import { describe, it, expect, beforeEach } from 'vitest';
import { projectState } from '../state/projectState.svelte';

describe('ProjectState Unit Tests', () => {
  beforeEach(() => {
    projectState.loadDefaultRules();
  });

  it('computes spoken text correctly with dictionary rules', () => {
    const raw = 'Google và TTS là công cụ tuyệt vời.';
    const computed = projectState.computeSpokenText(raw);
    expect(computed).toContain('Gú-gồ');
    expect(computed).toContain('Ti-ti-ép');
  });

  it('expands currency formats correctly', () => {
    const raw = 'Giá là 5.000.000 đ';
    const computed = projectState.computeSpokenText(raw);
    expect(computed).toContain('5 triệu đồng');
  });

  it('allows adding custom dictionary rules', () => {
    projectState.addDictionaryRule('AI', 'Trí tuệ nhân tạo');
    const computed = projectState.computeSpokenText('Ứng dụng AI');
    expect(computed).toContain('Trí tuệ nhân tạo');
  });
});
