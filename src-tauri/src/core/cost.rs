//! This month's cost: the Code-tab estimate from local transcripts plus the
//! extra-usage spend the usage endpoint reports.
//!
//! Transcripts are private. They are read in memory, only their token counts,
//! model, ids and timestamp are kept, and none of it is ever logged, stored or
//! sent anywhere. The only network call here fetches the public LiteLLM price
//! list.

mod heatmap;

pub use heatmap::{DayAccount, DayValues, Heatmap, HeatmapDay};

use super::accounts::Status;
use super::pooled::{days_from_civil, rfc3339_ms};
use super::ports::HttpRequest;
use super::{Account, Core};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const PRICES_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
const SESSIONS_DIR: &str = "claude-code-sessions";
/// Prices change rarely: one fetch serves an hour of refreshes.
const PRICES_TTL: Duration = Duration::from_secs(3_600);
/// After a failed fetch, wait before asking again.
const PRICES_RETRY: Duration = Duration::from_secs(60);
const DAY_MS: i64 = 86_400_000;

/// Why there is no dollar estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoEstimate {
    /// The price list could not be fetched, so tokens cannot be priced.
    PricesUnavailable,
}

/// One account's month. `estimate_usd` and `total_usd` are `None` when there
/// is no estimate: never a made-up zero.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCost {
    pub id: String,
    pub label: String,
    /// The account's badge colour, `#rrggbb`.
    pub colour: String,
    /// Code-tab estimate, month to date.
    pub estimate_usd: Option<f64>,
    /// Extra-usage spend, month to date (zero when the endpoint reports none).
    pub extra_usd: f64,
    pub total_usd: Option<f64>,
    /// This account's part of the month total, 0 to 1.
    pub share: Option<f64>,
    /// Tokens this account used this month, every class.
    pub tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthCost {
    /// Sum of the accounts' Code-tab estimates; labelled "est." in the UI.
    pub estimate_usd: Option<f64>,
    pub no_estimate: Option<NoEstimate>,
    /// Sum of the accounts' extra-usage spend.
    pub extra_usd: f64,
    /// Estimate plus extra usage; the accounts' totals add up to it.
    pub total_usd: Option<f64>,
    /// Messages left out of the estimate because their model has no price.
    pub unpriced_messages: usize,
    /// Tokens used this month across accounts, every class.
    pub tokens: u64,
    /// Biggest first.
    pub accounts: Vec<AccountCost>,
}

/// The cached outcome of a price fetch.
pub(super) struct PriceCache {
    fetched_at_ms: i64,
    prices: Option<Arc<Prices>>,
}

impl Core {
    /// This month's dollars (local calendar month): the Code-tab estimate plus
    /// extra-usage spend, in total and per account.
    pub fn month_cost(&self) -> MonthCost {
        let accounts: Vec<Account> = self
            .known_accounts()
            .unwrap_or_default()
            .into_iter()
            .filter(|a| a.status != Status::Missing)
            .collect();
        let this_month = self.local_month(self.ports.clock.now());

        let owners = session_owners(&accounts);
        let records = self.month_records(&owners, this_month);
        let mut tokens_by_account: HashMap<String, u64> = HashMap::new();
        for (id, record) in &records {
            *tokens_by_account.entry(id.clone()).or_default() += record.usage.total();
        }

        // With nothing to price there is nothing to fetch: the estimate is
        // truly zero. Otherwise no prices means no estimate, never zero.
        let (estimates, unpriced_messages, no_estimate) = if records.is_empty() {
            (Some(HashMap::new()), 0, None)
        } else {
            match self.prices() {
                Some(prices) => {
                    let (by_account, unpriced) = price_records(&records, &prices);
                    (Some(by_account), unpriced, None)
                }
                None => (None, 0, Some(NoEstimate::PricesUnavailable)),
            }
        };

        let mut rows: Vec<AccountCost> = accounts
            .iter()
            .map(|a| {
                let extra_usd = extra_usd_of(a);
                let estimate_usd = estimates
                    .as_ref()
                    .map(|by_account| by_account.get(&a.id).copied().unwrap_or(0.0));
                AccountCost {
                    id: a.id.clone(),
                    label: a.label.clone(),
                    colour: a.colour.clone(),
                    estimate_usd,
                    extra_usd,
                    total_usd: estimate_usd.map(|e| e + extra_usd),
                    share: None,
                    tokens: tokens_by_account.get(&a.id).copied().unwrap_or(0),
                }
            })
            .collect();

        let extra_usd = rows.iter().map(|r| r.extra_usd).sum();
        let estimate_usd = estimates
            .is_some()
            .then(|| rows.iter().filter_map(|r| r.estimate_usd).sum());
        let total_usd: Option<f64> = estimates
            .is_some()
            .then(|| rows.iter().filter_map(|r| r.total_usd).sum());
        if let Some(total) = total_usd.filter(|t| *t > 0.0) {
            for row in &mut rows {
                row.share = row.total_usd.map(|t| t / total);
            }
        }
        rows.sort_by(|a, b| {
            let dollars = |r: &AccountCost| r.total_usd.unwrap_or(r.extra_usd);
            dollars(b)
                .total_cmp(&dollars(a))
                .then_with(|| a.label.cmp(&b.label))
        });

        MonthCost {
            estimate_usd,
            no_estimate,
            extra_usd,
            total_usd,
            unpriced_messages,
            tokens: rows.iter().map(|r| r.tokens).sum(),
            accounts: rows,
        }
    }

    /// `(year, month)` of `at` on the local calendar.
    fn local_month(&self, at: SystemTime) -> (i64, i64) {
        let offset = i64::from(self.ports.clock.utc_offset_secs(at));
        civil_month(epoch_ms(at) + offset * 1_000)
    }

    /// Every usage record of this month in the mapped transcripts, one per
    /// message, with the account that owns it.
    fn month_records(
        &self,
        owners: &HashMap<String, String>,
        this_month: (i64, i64),
    ) -> Vec<(String, Record)> {
        self.records_where(owners, self.month_start_ms(this_month), |at| {
            self.local_month(at) == this_month
        })
    }

    /// Every usage record `keep` accepts in the mapped transcripts, one per
    /// message, with the account that owns it. Files untouched since a day
    /// before `since_ms` are not read.
    fn records_where(
        &self,
        owners: &HashMap<String, String>,
        since_ms: i64,
        keep: impl Fn(SystemTime) -> bool,
    ) -> Vec<(String, Record)> {
        let root = self.ports.roots.home.join(".claude").join("projects");
        let oldest_mtime_ms = since_ms - DAY_MS;
        let mut files = transcript_files(&root, owners);
        files.sort();

        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut kept: Vec<(String, Record)> = Vec::new();
        for (path, account) in files {
            // A file untouched since before the month holds nothing from it.
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .map(epoch_ms);
            if modified.is_ok_and(|at| at < oldest_mtime_ms) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in text.lines().filter(|l| l.contains("\"usage\"")) {
                let Some(record) = parse_record(line) else {
                    continue;
                };
                let at =
                    UNIX_EPOCH + Duration::from_millis(u64::try_from(record.at_ms).unwrap_or(0));
                if !keep(at) {
                    continue;
                }
                match seen.get(&record.key) {
                    // A streamed message repeats with growing output: the last
                    // word is the largest.
                    Some(&i) => {
                        if record.usage.output > kept[i].1.usage.output {
                            kept[i].1.usage = record.usage;
                        }
                    }
                    None => {
                        seen.insert(record.key.clone(), kept.len());
                        kept.push((account.clone(), record));
                    }
                }
            }
        }
        kept
    }

    /// Epoch ms of local midnight on the first of `month`, near enough to
    /// prefilter files by modified time.
    fn month_start_ms(&self, (year, month): (i64, i64)) -> i64 {
        let utc_midnight = days_from_civil(year, month, 1) * DAY_MS;
        let at = UNIX_EPOCH + Duration::from_millis(u64::try_from(utc_midnight).unwrap_or(0));
        utc_midnight - i64::from(self.ports.clock.utc_offset_secs(at)) * 1_000
    }

    /// The price list, fetched online (cached for an hour); `None` when it
    /// cannot be fetched.
    fn prices(&self) -> Option<Arc<Prices>> {
        let mut cache = self.prices.lock().unwrap();
        let now = epoch_ms(self.ports.clock.now());
        if let Some(c) = cache.as_ref() {
            let ttl = if c.prices.is_some() {
                PRICES_TTL
            } else {
                PRICES_RETRY
            };
            if now - c.fetched_at_ms < i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX) {
                return c.prices.clone();
            }
        }
        let prices = self
            .ports
            .http
            .send(HttpRequest::get(PRICES_URL))
            .ok()
            .filter(|r| r.status == 200)
            .and_then(|r| Prices::parse(&r.body))
            .map(Arc::new);
        *cache = Some(PriceCache {
            fetched_at_ms: now,
            prices: prices.clone(),
        });
        prices
    }
}

/// Extra-usage spend in dollars; an absent field means none, and so does an
/// account whose extra usage is not enabled.
fn extra_usd_of(account: &Account) -> f64 {
    account
        .usage
        .as_ref()
        .and_then(|u| u.extra_usage.as_ref())
        .filter(|e| e.enabled)
        .and_then(|e| e.used_minor)
        .map_or(0.0, |cents| cents as f64 / 100.0)
}

fn epoch_ms(at: SystemTime) -> i64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// `(year, month)` of a count of milliseconds since 1970-01-01 on a calendar.
fn civil_month(ms: i64) -> (i64, i64) {
    let (year, month, _) = civil_date(ms.div_euclid(DAY_MS));
    (year, month)
}

/// `(year, month, day)` of a count of days since 1970-01-01.
fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let day = doy - (153 * mp + 2) / 5 + 1;
    (year, month, day)
}

// Mapping transcripts to accounts

/// Session id -> owning account id, from each profile's `claude-code-sessions`.
fn session_owners(accounts: &[Account]) -> HashMap<String, String> {
    let mut owners = HashMap::new();
    for account in accounts {
        for session in session_ids(&account.path.join(SESSIONS_DIR)) {
            owners.entry(session).or_insert_with(|| account.id.clone());
        }
    }
    owners
}

/// Every `cliSessionId` and `priorCliSessionIds` in the session files below `dir`.
fn session_ids(dir: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                collect_session_ids(&path, &mut ids);
            }
        }
    }
    ids
}

fn collect_session_ids(file: &Path, ids: &mut HashSet<String>) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if let Some(id) = json.get("cliSessionId").and_then(Value::as_str) {
        ids.insert(id.to_string());
    }
    if let Some(prior) = json.get("priorCliSessionIds").and_then(Value::as_array) {
        ids.extend(prior.iter().filter_map(Value::as_str).map(str::to_string));
    }
}

/// The transcripts of mapped sessions, with their account. A transcript is
/// `projects/{project}/{session}.jsonl`; a session's subagents write under
/// `projects/{project}/{session}/`.
fn transcript_files(root: &Path, owners: &HashMap<String, String>) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    let Ok(projects) = std::fs::read_dir(root) else {
        return files;
    };
    for project in projects.flatten().filter(|p| p.path().is_dir()) {
        let Ok(entries) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let session = name.strip_suffix(".jsonl").unwrap_or(&name);
            if let Some(account) = owners.get(session) {
                collect_jsonl(&entry.path(), account, &mut files);
            }
        }
    }
    files
}

fn collect_jsonl(path: &Path, account: &str, files: &mut Vec<(PathBuf, String)>) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect_jsonl(&entry.path(), account, files);
            }
        }
    } else if path.extension().is_some_and(|e| e == "jsonl") {
        files.push((path.to_path_buf(), account.to_string()));
    }
}

// Reading usage records

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TokenUsage {
    input: u64,
    output: u64,
    cache_5m: u64,
    cache_1h: u64,
    cache_read: u64,
}

impl TokenUsage {
    fn is_empty(&self) -> bool {
        *self == TokenUsage::default()
    }

    /// Every token of every class, cache reads and writes included.
    fn total(&self) -> u64 {
        self.input + self.output + self.cache_5m + self.cache_1h + self.cache_read
    }
}

struct Record {
    /// Message id plus request id, or the message id alone.
    key: String,
    model: String,
    at_ms: i64,
    usage: TokenUsage,
}

/// One transcript line's usage record; `None` for anything else.
fn parse_record(line: &str) -> Option<Record> {
    let json: Value = serde_json::from_str(line).ok()?;
    let message = json.get("message")?;
    let usage = message.get("usage")?.as_object()?;
    let message_id = message.get("id")?.as_str()?;
    let key = match json.get("requestId").and_then(Value::as_str) {
        Some(request) if !request.is_empty() => format!("{message_id}:{request}"),
        _ => message_id.to_string(),
    };
    let count = |v: Option<&Value>| v.and_then(Value::as_u64).unwrap_or(0);
    // Without the 5-minute / 1-hour split, every cache write is a 5-minute one.
    let (cache_5m, cache_1h) = match usage.get("cache_creation") {
        Some(split) => (
            count(split.get("ephemeral_5m_input_tokens")),
            count(split.get("ephemeral_1h_input_tokens")),
        ),
        None => (count(usage.get("cache_creation_input_tokens")), 0),
    };
    Some(Record {
        key,
        model: message.get("model")?.as_str()?.to_string(),
        at_ms: rfc3339_ms(json.get("timestamp")?.as_str()?)?,
        usage: TokenUsage {
            input: count(usage.get("input_tokens")),
            output: count(usage.get("output_tokens")),
            cache_5m,
            cache_1h,
            cache_read: count(usage.get("cache_read_input_tokens")),
        },
    })
}

// Pricing

/// Dollars per token, by token class.
#[derive(Debug, Clone, Copy)]
struct Price {
    input: f64,
    output: f64,
    cache_5m: f64,
    cache_1h: f64,
    cache_read: f64,
}

impl Price {
    fn cost(&self, t: &TokenUsage) -> f64 {
        t.input as f64 * self.input
            + t.output as f64 * self.output
            + t.cache_5m as f64 * self.cache_5m
            + t.cache_1h as f64 * self.cache_1h
            + t.cache_read as f64 * self.cache_read
    }
}

/// LiteLLM's model price list, reduced to the models with token prices.
struct Prices(HashMap<String, Price>);

impl Prices {
    fn parse(body: &str) -> Option<Prices> {
        let json: Value = serde_json::from_str(body).ok()?;
        let models: HashMap<String, Price> = json
            .as_object()?
            .iter()
            .filter_map(|(name, entry)| Some((name.clone(), parse_price(entry)?)))
            .collect();
        (!models.is_empty()).then_some(Prices(models))
    }

    /// The price for a transcript's model name: exact, under the `anthropic/`
    /// prefix, or without a trailing `-YYYYMMDD` date.
    fn of(&self, model: &str) -> Option<&Price> {
        let undated = model
            .rsplit_once('-')
            .filter(|(_, tail)| tail.len() == 8 && tail.bytes().all(|b| b.is_ascii_digit()))
            .map(|(head, _)| head);
        [Some(model), undated]
            .into_iter()
            .flatten()
            .find_map(|name| {
                self.0
                    .get(name)
                    .or_else(|| self.0.get(&format!("anthropic/{name}")))
            })
    }
}

fn parse_price(entry: &Value) -> Option<Price> {
    let field = |name: &str| entry.get(name).and_then(Value::as_f64);
    let input = field("input_cost_per_token")?;
    let output = field("output_cost_per_token")?;
    // Anthropic's multiples of the input price, for entries that omit a cache price.
    Some(Price {
        input,
        output,
        cache_5m: field("cache_creation_input_token_cost").unwrap_or(input * 1.25),
        cache_1h: field("cache_creation_input_token_cost_above_1hr").unwrap_or(input * 2.0),
        cache_read: field("cache_read_input_token_cost").unwrap_or(input * 0.1),
    })
}

/// Dollars per account, and how many messages had no price.
fn price_records(records: &[(String, Record)], prices: &Prices) -> (HashMap<String, f64>, usize) {
    let mut by_account: HashMap<String, f64> = HashMap::new();
    let mut unpriced = 0;
    for (account, record) in records {
        match prices.of(&record.model) {
            Some(price) => {
                *by_account.entry(account.clone()).or_default() += price.cost(&record.usage);
            }
            None if !record.usage.is_empty() => unpriced += 1,
            None => {}
        }
    }
    (by_account, unpriced)
}

/// One message's dollars; `None` when its model has no price.
fn price_of(prices: &Prices, record: &Record) -> Option<f64> {
    prices.of(&record.model).map(|p| p.cost(&record.usage))
}
