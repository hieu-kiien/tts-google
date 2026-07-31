<script lang="ts">
  export interface QuickstartTemplate {
    id: string;
    icon: string;
    title: string;
    description: string;
    defaultVoice: string;
    preset: string;
    speakingRate: number;
    sampleText: string;
  }

  let {
    show = false,
    onSelectTemplate,
    onClose,
  }: {
    show: boolean;
    onSelectTemplate: (template: QuickstartTemplate) => void;
    onClose: () => void;
  } = $props();

  const templates: QuickstartTemplate[] = [
    {
      id: "assistant",
      icon: "🎧",
      title: "The Everyday Assistant",
      description: "Giọng trợ lý cá nhân chuyên nghiệp, rõ ràng, hữu ích.",
      defaultVoice: "Kore",
      preset: "Trang trọng (Audio Profile: Professional Assistant, Tone: Helpful & Crisp)",
      speakingRate: 1.0,
      sampleText: "Xin chào! Tôi có thể giúp gì cho công việc hôm nay của bạn?",
    },
    {
      id: "npc",
      icon: "🎮",
      title: "The Guarded NPC",
      description: "Tạo kịch bản đối thoại đa nhân vật thế giới game fantasy.",
      defaultVoice: "Fenrir",
      preset: "Kể chuyện (Audio Profile: Guarded Fantasy NPC, Tone: Suspicious & Deep)",
      speakingRate: 0.9,
      sampleText: "Dừng lại! Người lạ mặt, ngươi tìm kiếm điều gì tại vùng đất cấm này?",
    },
    {
      id: "cohost",
      icon: "🎙️",
      title: "The Energetic Co-Host",
      description: "Giọng giao tiếp Podcast sôi nổi, hào hứng, tự nhiên.",
      defaultVoice: "Puck",
      preset: "Tự nhiên (Audio Profile: Energetic Podcast Co-Host, Tone: Dynamic & Casual)",
      speakingRate: 1.1,
      sampleText: "Chào mừng các bạn quay trở lại với tập Podcast hôm nay! Chủ đề cực hot này chắc chắn sẽ làm bạn bất ngờ đấy!",
    },
    {
      id: "storyteller",
      icon: "📖",
      title: "The Master Storyteller",
      description: "Giọng đọc sách nói truyền cảm, nghệ thuật kể chuyện cuốn hút.",
      defaultVoice: "Aoede",
      preset: "Kể chuyện (Audio Profile: Master Storyteller, Tone: Cinematic & Expressive)",
      speakingRate: 0.95,
      sampleText: "Ngày xửa ngày xưa, tại một vương quốc xa xôi nằm giữa những rặng núi mờ sương...",
    },
    {
      id: "ad",
      icon: "📢",
      title: "The Ad Voiceover",
      description: "Giọng đọc quảng cáo mượt mà, thuyết phục, đẳng cấp cao.",
      defaultVoice: "Orpheus",
      preset: "Quảng cáo (Audio Profile: Premium Commercial Announcer, Tone: Smooth & Persuasive)",
      speakingRate: 1.05,
      sampleText: "Trải nghiệm đỉnh cao công nghệ cùng giải pháp hoàn toàn mới ngay hôm nay!",
    },
    {
      id: "guide",
      icon: "💼",
      title: "The Training Guide",
      description: "Giọng hướng dẫn doanh nghiệp rõ ràng, uy quyền, chuẩn xác.",
      defaultVoice: "Pegasus",
      preset: "Giáo dục (Audio Profile: Authoritative Corporate Trainer, Tone: Clear & Structured)",
      speakingRate: 1.0,
      sampleText: "Sau đây là 3 quy trình cốt lõi bạn cần nắm vững trong khóa đào tạo này.",
    },
    {
      id: "host",
      icon: "🌟",
      title: "The Game Show Host",
      description: "Giọng MC Gameshow truyền hình kịch tính, năng lượng bùng nổ.",
      defaultVoice: "Zephyr",
      preset: "Tin tức (Audio Profile: Vibrant Game Show Host, Tone: Theatrical & High Energy)",
      speakingRate: 1.15,
      sampleText: "Và câu trả lời chính xác trị giá 100 triệu đồng thuộc về... người chơi số 2!",
    },
    {
      id: "teacher",
      icon: "👨‍🏫",
      title: "The Patient Teacher",
      description: "Giọng giáo viên giảng bài kiên nhẫn, động viên học viên.",
      defaultVoice: "Calliope",
      preset: "Giáo dục (Audio Profile: Encouraging Language Tutor, Tone: Patient & Warm)",
      speakingRate: 0.9,
      sampleText: "Đừng lo lắng! Hãy lắng nghe lại câu này một lần nữa và phát âm cùng cô nhé.",
    },
  ];
</script>

{#if show}
  <div 
    class="modal-backdrop" tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) onClose?.(); }}
    onkeydown={(e) => { if (e.key === 'Escape') onClose?.(); }}
    role="presentation"
  >
    <div 
      class="modal-content" 
      role="dialog" 
      aria-modal="true" 
      aria-labelledby="templates-title"
    >
      <div class="modal-header">
        <h3 id="templates-title">🎨 Google AI Studio Quickstart Templates</h3>
        <button class="close-btn" onclick={onClose} aria-label="Đóng bảng templates">✕</button>
      </div>

      <div class="modal-body">
        <p class="subtitle">
          Chọn 1 mẫu Đạo Diễn Giọng Đọc (Audio Profile & Tone) chuẩn từ Google AI Studio để tự động cấu hình giọng và phong cách:
        </p>

        <div class="templates-grid">
          {#each templates as t}
            <button class="template-card" onclick={() => { onSelectTemplate(t); onClose(); }}>
              <div class="card-top">
                <span class="card-icon">{t.icon}</span>
                <span class="card-title">{t.title}</span>
              </div>
              <p class="card-desc">{t.description}</p>
              <div class="card-footer">
                <span class="voice-badge">🎙️ Voice: {t.defaultVoice}</span>
                <span class="speed-badge">⚡ {t.speakingRate}x</span>
              </div>
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }
  .modal-content {
    background: var(--color-bg-surface);
    border-radius: 12px;
    width: 90%;
    max-width: 820px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-lg, 0 20px 25px -5px rgba(0, 0, 0, 0.1));
    overflow: hidden;
  }
  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--color-bg-surface-raised);
  }
  .modal-header h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: var(--color-text-primary);
  }
  .close-btn {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: var(--color-text-muted);
  }
  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--color-text-secondary);
  }
  .templates-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 14px;
  }
  .template-card {
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s ease;
  }
  .template-card:hover {
    border-color: var(--color-accent);
    box-shadow: var(--shadow-md, 0 4px 12px rgba(59, 130, 246, 0.15));
    transform: translateY(-2px);
  }
  .card-top {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .card-icon {
    font-size: 20px;
  }
  .card-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--color-text-primary);
  }
  .card-desc {
    font-size: 12px;
    color: var(--color-text-muted);
    margin: 0;
    line-height: 1.4;
  }
  .card-footer {
    display: flex;
    gap: 10px;
    margin-top: 4px;
  }
  .voice-badge, .speed-badge {
    font-size: 11px;
    font-weight: 600;
    background: var(--color-bg-surface-raised);
    color: var(--color-text-secondary);
    padding: 2px 6px;
    border-radius: 4px;
  }
</style>
