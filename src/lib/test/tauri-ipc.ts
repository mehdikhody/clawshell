import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks';

export type IpcCall = { cmd: string; args: unknown };
export type IpcHandlers = Record<string, (args: unknown) => unknown>;

/**
 * UI test harness: stands in for the Tauri IPC boundary.
 * Commands answer from `handlers` (canned core results) and every call is recorded.
 * Call `restore()` in afterEach.
 */
export function mockTauri(handlers: IpcHandlers = {}) {
	const calls: IpcCall[] = [];
	mockWindows('main');
	mockIPC((cmd, args) => {
		calls.push({ cmd, args });
		return handlers[cmd]?.(args);
	});
	return { calls, restore: clearMocks };
}
