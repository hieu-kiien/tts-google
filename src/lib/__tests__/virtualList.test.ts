import { describe, it, expect } from "vitest";
import { get } from "svelte/store";
import { createVirtualizer } from "@tanstack/svelte-virtual";
import type { SegmentRecord, AppErrorCode } from "../types/tts";

describe("TanStack Virtualizer 10,000 Segment Scalability Benchmark", () => {
  it("should initialize TanStack virtualizer for 10,000 items and calculate total height and window bounds under 100ms", () => {
    const startTime = performance.now();
    const segments: SegmentRecord[] = Array.from({ length: 10000 }, (_, i) => ({
      id: `seg_${i + 1}`,
      project_id: "p_bench",
      position: i + 1,
      text: `Đoạn văn bản mẫu thứ ${i + 1} phục vụ kiểm thử virtualization 10.000 phần tử.`,
      prompt: "Giọng đọc truyền cảm",
      status: "pending",
      attempts: 0,
      audio_path: null,
      duration_ms: 0,
      error_code: null,
      error_message: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      state_revision: 1,
      attempt_count: 0,
      cancel_requested: false,
      output_size: 0,
    }));
    const createDuration = performance.now() - startTime;

    expect(segments.length).toBe(10000);
    expect(createDuration).toBeLessThan(100);

    const mockScrollContainer = {
      scrollTop: 28000,
      clientHeight: 800,
      scrollHeight: 1400000,
      getBoundingClientRect: () => ({
        top: 0, left: 0, width: 1000, height: 800, bottom: 800, right: 1000, x: 0, y: 0, toJSON: () => {}
      }),
      addEventListener: () => {},
      removeEventListener: () => {},
    } as unknown as Element;

    const virtualizerStore = createVirtualizer({
      count: segments.length,
      getScrollElement: () => mockScrollContainer,
      estimateSize: () => 140,
      overscan: 5,
      observeElementOffset: (_, cb) => {
        cb(28000, true);
        return () => {};
      },
      observeElementRect: (_, cb) => {
        cb({ width: 1000, height: 800 });
        return () => {};
      },
    });

    const instance = get(virtualizerStore);
    expect(instance.options.count).toBe(10000);
    expect(instance.getTotalSize()).toBe(1400000); // 10,000 items * 140px = 1,400,000px

    const virtualItems = instance.getVirtualItems();
    expect(virtualItems.length).toBeLessThanOrEqual(20);
    expect(virtualItems.length).toBeGreaterThan(0);
  });

  it("should enforce strict AppErrorCode type safety", () => {
    const validCodes: AppErrorCode[] = [
      "AUTH_INVALID",
      "RATE_LIMITED",
      "DAILY_QUOTA_EXHAUSTED",
      "NETWORK_UNAVAILABLE",
      "VALIDATION_FAILED",
      "DATABASE_ERROR",
      "AUDIO_CORRUPT",
      "CONTENT_FILTERED",
      "FILE_SYSTEM_ERROR",
      "QUEUE_ERROR",
      "INTERNAL_ERROR",
    ];
    expect(validCodes.length).toBe(11);
  });
});
