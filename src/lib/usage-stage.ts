import type { Settings } from '$lib/settings';

/** How close a window is to its limit: the bar warms to red as it fills. */
export type UsageStage = 'normal' | 'warning' | 'limit';

/** The two thresholds a bar's colour depends on. */
export type UsageThresholds = Pick<Settings, 'warningThreshold' | 'limitThreshold'>;

/** The defaults the core starts with, used until the saved settings are read. */
export const defaultThresholds: UsageThresholds = { warningThreshold: 80, limitThreshold: 100 };

/** Stage for a used percentage: at the limit threshold it is `limit`, at the warning threshold `warning`. */
export function usageStage(percent: number, thresholds: UsageThresholds): UsageStage {
	if (percent >= thresholds.limitThreshold) return 'limit';
	if (percent >= thresholds.warningThreshold) return 'warning';
	return 'normal';
}

/** Classes that recolour a Progress bar's fill for a stage; `normal` keeps the primary colour. */
export const usageBarClass: Record<UsageStage, string> = {
	normal: '',
	warning: '*:data-[slot=progress-indicator]:bg-usage-warning',
	limit: '*:data-[slot=progress-indicator]:bg-destructive'
};
