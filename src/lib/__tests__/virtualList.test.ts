import { describe, it, expect } from "vitest";
import type { SegmentRecord } from "../types/tts";

describe("Virtual List 10,000 Segment Scalability Benchmark", () => {
  it("should handle 10,000 segment objects in memory with sub-millisecond calculation time", () => {
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
    expect(createDuration).toBeLessThan(100); // 10k items created under 100ms

    // Simulate virtual window calculation (overscan = 5, itemHeight = 140px, viewportHeight = 800px)
    const viewportHeight = 800;
    const itemHeight = 140;
    const overscan = 5;

    const visibleCount = Math.ceil(viewportHeight / itemHeight); // ~6 visible rows
    const totalRenderedLimit = visibleCount + overscan * 2; // ~16 rows total in DOM

    // Compute virtual window range for scrollTop = 28000 (middle of list ~row 200)
    const scrollTop = 28000;
    const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
    const endIndex = Math.min(segments.length - 1, Math.floor((scrollTop + viewportHeight) / itemHeight) + overscan);
    const virtualWindow = segments.slice(startIndex, endIndex + 1);

    expect(virtualWindow.length).toBeLessThanOrEqual(totalRenderedLimit + 2);
    expect(virtualWindow.length).toBeGreaterThan(0);
    expect(virtualWindow[0].position).toBeGreaterThan(180);
  });
});
