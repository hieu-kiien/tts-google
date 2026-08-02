// Test setup — Mock Tauri IPC layer for frontend unit tests
import { vi } from 'vitest';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
	convertFileSrc: vi.fn((path: string) => `asset://localhost/${path}`),
}));

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(() => Promise.resolve(() => {})),
	emit: vi.fn(),
}));

// Mock @tauri-apps/api/window
vi.mock('@tauri-apps/api/window', () => ({
	getCurrentWindow: vi.fn(() => ({
		onCloseRequested: vi.fn(() => Promise.resolve(() => {})),
		destroy: vi.fn(() => Promise.resolve()),
	})),
}));

// Mock @tauri-apps/plugin-opener
vi.mock('@tauri-apps/plugin-opener', () => ({
	open: vi.fn(),
}));
