import { describe, it, expect, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

describe('Vitest Setup Smoke Test', () => {
	it('should have Tauri IPC mocked', () => {
		expect(vi.isMockFunction(invoke)).toBe(true);
	});

	it('should be able to mock invoke return values', async () => {
		const mockInvoke = vi.mocked(invoke);
		mockInvoke.mockResolvedValueOnce({ id: 'test-project', name: 'Test' });

		const result = await invoke('get_project', { id: 'test-project' });
		expect(result).toEqual({ id: 'test-project', name: 'Test' });
		expect(mockInvoke).toHaveBeenCalledWith('get_project', { id: 'test-project' });
	});
});
