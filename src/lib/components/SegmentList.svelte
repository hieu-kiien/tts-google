<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import type { SegmentRecord } from "../types/tts";
  import SegmentItem from "./SegmentItem.svelte";
  import { projectState } from "../state/projectState.svelte";
  import { playerState } from "../state/playerState.svelte";

  interface Props {
    segments: SegmentRecord[];
    selectedSegmentIds: Set<string>;
    isSynthesizingPreview: boolean;
    autoScrollEnabled?: boolean;
    registerTextarea?: (id: string, el: HTMLTextAreaElement | null) => void;
    onSelectSegment: (id: string, e: MouseEvent) => void;
    onSegmentClick: (seg: SegmentRecord) => void;
    onSegmentTextInput: (seg: SegmentRecord) => void;
    onPlayPreview: (seg: SegmentRecord) => void;
    onResynthesizeSegment: (seg: SegmentRecord) => void;
    onSplitSegment: (seg: SegmentRecord) => void;
    onMoveSegment: (seg: SegmentRecord, direction: 'up' | 'down') => void;
    onInsertSegmentBelow: (seg: SegmentRecord) => void;
    onMergeWithPrevious: (seg: SegmentRecord) => void;
    onDeleteSingleSegment: (seg: SegmentRecord) => void;
  }

  let {
    segments,
    selectedSegmentIds,
    isSynthesizingPreview,
    autoScrollEnabled = true,
    registerTextarea,
    onSelectSegment,
    onSegmentClick,
    onSegmentTextInput,
    onPlayPreview,
    onResynthesizeSegment,
    onSplitSegment,
    onMoveSegment,
    onInsertSegmentBelow,
    onMergeWithPrevious,
    onDeleteSingleSegment
  }: Props = $props();

  let scrollContainerEl = $state<HTMLDivElement | null>(null);

  // TanStack Virtualizer for Svelte
  let virtualizer = $derived.by(() => {
    return createVirtualizer({
      count: segments.length,
      getScrollElement: () => scrollContainerEl,
      estimateSize: () => 140, // Estimated pixel height of each segment row
      overscan: 5, // Extra rows above and below viewport for smooth scrolling
    });
  });

  function measureNode(node: HTMLElement) {
    $virtualizer.measureElement(node);
    return {
      destroy() {
        $virtualizer.measureElement(null);
      }
    };
  }

  // Auto-scroll to active playing segment only if autoScrollEnabled is true
  $effect(() => {
    if (autoScrollEnabled && playerState.currentPlayingSegmentId) {
      const idx = segments.findIndex(s => s.id === playerState.currentPlayingSegmentId);
      if (idx !== -1) {
        $virtualizer.scrollToIndex(idx, { align: "auto" });
      }
    }
  });
</script>

<div 
  class="virtual-scroll-container" 
  bind:this={scrollContainerEl}
  style="height: 100%; overflow-y: auto; position: relative;"
>
  <div
    class="virtual-scroll-inner"
    style="height: {$virtualizer.getTotalSize()}px; width: 100%; position: relative;"
  >
    {#each $virtualizer.getVirtualItems() as virtualRow (virtualRow.key)}
      {@const seg = segments[virtualRow.index]}
      {#if seg}
        <div
          data-index={virtualRow.index}
          use:measureNode
          style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({virtualRow.start}px);"
        >
          <SegmentItem
            {seg}
            isSelected={selectedSegmentIds.has(seg.id)}
            isActive={projectState.activeSegmentId === seg.id}
            isPlaying={playerState.currentPlayingSegmentId === seg.id}
            {isSynthesizingPreview}
            totalSegments={segments.length}
            {registerTextarea}
            onSelect={onSelectSegment}
            onClick={onSegmentClick}
            onTextInput={onSegmentTextInput}
            {onPlayPreview}
            onResynthesize={onResynthesizeSegment}
            onSplit={onSplitSegment}
            onMove={onMoveSegment}
            onInsertBelow={onInsertSegmentBelow}
            onMerge={onMergeWithPrevious}
            onDelete={onDeleteSingleSegment}
          />
        </div>
      {/if}
    {/each}
  </div>
</div>
