<script lang="ts">
  import { projectState } from "../state/projectState.svelte";
  import { uiState } from "../state/uiState.svelte";
  import type { ProjectRecord } from "../types/tts";
  import { createProject, deleteProject, getProjectSegments } from "../api/projectClient";
  import { invoke } from "@tauri-apps/api/core";
  import { toastStore } from "../state/toasts.svelte";
  import { getErrorMessage } from "../utils/errorUtils";

  let searchQuery = $state("");
  let sortBy = $state<"updated" | "name" | "progress">("updated");

  // Multi-selection state for batch project deletion
  let selectedProjectIds = $state<Set<string>>(new Set());

  let filteredProjects = $derived.by<ProjectRecord[]>(() => {
    let list = [...projectState.projects];
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(p => p.name.toLowerCase().includes(q));
    }
    list.sort((a, b) => {
      if (sortBy === "name") return a.name.localeCompare(b.name);
      if (sortBy === "progress") {
        const progA = (a.completed_count || 0) / (a.segment_count || 1);
        const progB = (b.completed_count || 0) / (b.segment_count || 1);
        return progB - progA;
      }
      return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
    });
    return list;
  });

  const isAllSelected = $derived.by(() => {
    if (filteredProjects.length === 0) return false;
    return filteredProjects.every(p => selectedProjectIds.has(p.id));
  });

  function toggleSelectAll() {
    if (isAllSelected) {
      selectedProjectIds.clear();
    } else {
      selectedProjectIds = new Set(filteredProjects.map(p => p.id));
    }
  }

  function toggleSelectProject(id: string, e: MouseEvent | Event) {
    e.stopPropagation();
    const next = new Set(selectedProjectIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedProjectIds = next;
  }

  async function selectProject(proj: ProjectRecord) {
    projectState.currentProject = proj;
    try {
      const segs = await getProjectSegments(proj.id);
      projectState.segments = segs;
    } catch (err: unknown) {
      console.warn("Lỗi load segments:", getErrorMessage(err));
      projectState.segments = [];
    }
    uiState.activeView = "editor";
  }

  async function createNewProject() {
    try {
      const newProj = await createProject({
        name: `Dự án TTS ${new Date().toLocaleDateString('vi-VN')}`,
        sourceText: "Nhập nội dung văn bản tiếng Việt của bạn tại đây...",
        voice: "Kore",
        preset: "Tự nhiên",
      });
      projectState.projects = [newProj, ...projectState.projects];
      projectState.currentProject = newProj;
      const segs = await getProjectSegments(newProj.id);
      projectState.segments = segs;
      uiState.activeView = "editor";
    } catch (err: unknown) {
      toastStore.showError(`Lỗi tạo dự án: ${getErrorMessage(err)}`);
    }
  }

  async function handleDeleteSingleProject(proj: ProjectRecord, e: Event) {
    e.stopPropagation();
    if (!confirm(`Bạn có chắc chắn muốn xóa dự án "${proj.name}"? Tất cả các đoạn audio liên quan sẽ bị xóa vĩnh viễn.`)) {
      return;
    }
    try {
      await deleteProject(proj.id);
      projectState.projects = projectState.projects.filter(p => p.id !== proj.id);
      if (projectState.currentProject?.id === proj.id) {
        projectState.currentProject = null;
        projectState.segments = [];
      }
      const nextSel = new Set(selectedProjectIds);
      nextSel.delete(proj.id);
      selectedProjectIds = nextSel;
      toastStore.showSuccess(`Đã xóa dự án "${proj.name}" thành công.`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi xóa dự án: ${getErrorMessage(err)}`);
    }
  }

  async function handleDeleteSelectedProjects() {
    const ids = Array.from(selectedProjectIds);
    if (ids.length === 0) return;
    if (!confirm(`Bạn có chắc chắn muốn xóa ${ids.length} dự án đã chọn? Tất cả audio sẽ bị xóa vĩnh viễn.`)) {
      return;
    }
    try {
      await invoke("delete_projects_batch", { projectIds: ids });
      projectState.projects = projectState.projects.filter(p => !selectedProjectIds.has(p.id));
      if (projectState.currentProject && selectedProjectIds.has(projectState.currentProject.id)) {
        projectState.currentProject = null;
        projectState.segments = [];
      }
      selectedProjectIds.clear();
      toastStore.showSuccess(`Đã xóa ${ids.length} dự án thành công!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi xóa hàng loạt: ${getErrorMessage(err)}`);
    }
  }
</script>

<div class="project-library-container" role="region" aria-label="Thư viện dự án">
  <!-- Header Bar -->
  <header class="library-header">
    <div class="header-title">
      <h1>📚 Thư viện dự án TTS</h1>
      <span class="badge">Gemini Free Tier</span>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={() => uiState.showTemplatesModal = true} aria-label="Mẫu dự án Quickstart">
        🎨 Dự án Mẫu Quickstart
      </button>
      <button class="btn btn-secondary" onclick={() => uiState.showBatchProcessorModal = true} aria-label="Xử lý file hàng loạt">
        🚀 Xử lý hàng loạt
      </button>
      <button class="btn btn-secondary" onclick={() => uiState.showImportWizard = true} aria-label="Nhập file văn bản">
        📂 Nhập file / dán text
      </button>
      <button class="btn btn-primary" onclick={createNewProject} aria-label="Tạo dự án mới">
        ➕ Tạo dự án mới
      </button>
    </div>
  </header>

  <!-- Filter & Batch Action Toolbar -->
  <div class="library-toolbar">
    <div class="search-box">
      <span class="search-icon" aria-hidden="true">🔍</span>
      <input 
        type="text" 
        placeholder="Tìm kiếm dự án..." 
        bind:value={searchQuery}
        aria-label="Tìm kiếm dự án"
      />
    </div>

    <div class="toolbar-right">
      {#if selectedProjectIds.size > 0}
        <div class="batch-bar">
          <span class="batch-count">Đã chọn <strong>{selectedProjectIds.size}</strong> dự án</span>
          <button class="btn btn-danger" onclick={handleDeleteSelectedProjects}>
            🗑️ Xóa hàng loạt ({selectedProjectIds.size})
          </button>
        </div>
      {/if}

      {#if filteredProjects.length > 0}
        <label class="select-all-label" title="Chọn tất cả các dự án đang hiển thị">
          <input type="checkbox" checked={isAllSelected} onchange={toggleSelectAll} />
          Chọn tất cả
        </label>
      {/if}

      <div class="sort-box">
        <label for="sort-select">Sắp xếp theo:</label>
        <select id="sort-select" bind:value={sortBy} aria-label="Sắp xếp danh sách dự án">
          <option value="updated">Chỉnh sửa gần đây</option>
          <option value="name">Tên dự án (A-Z)</option>
          <option value="progress">Tiến độ hoàn tất</option>
        </select>
      </div>
    </div>
  </div>

  <!-- Projects Grid / Empty State -->
  {#if filteredProjects.length === 0}
    <div class="empty-state" role="status">
      <div class="empty-icon" aria-hidden="true">📖</div>
      <h2>Chưa có dự án nào</h2>
      <p>Ứng dụng giúp bạn biến sách, truyện, tài liệu tiếng Việt dài thành audio 24kHz chất lượng cao bằng giọng đọc Gemini Free Tier.</p>
      
      <div class="empty-actions">
        <button class="btn btn-primary btn-lg" onclick={() => uiState.showTemplatesModal = true}>
          🎨 Thử ngay Dự án Mẫu Quickstart
        </button>
        <button class="btn btn-secondary btn-lg" onclick={createNewProject}>
          ✨ Tạo dự án trắng mới
        </button>
        <button class="btn btn-secondary btn-lg" onclick={() => uiState.showImportWizard = true}>
          📄 Nhập PDF / Docx / Text
        </button>
      </div>
    </div>
  {:else}
    <div class="projects-grid">
      {#each filteredProjects as proj (proj.id)}
        <div 
          class="project-card {selectedProjectIds.has(proj.id) ? 'selected' : ''}" 
          onclick={() => selectProject(proj)}
          onkeydown={(e) => e.key === 'Enter' && selectProject(proj)}
          tabindex="0"
          role="button"
          aria-label={`Dự án ${proj.name}`}
        >
          <div class="card-header">
            <div class="card-title-box">
              <input 
                type="checkbox" 
                class="card-checkbox"
                checked={selectedProjectIds.has(proj.id)} 
                onclick={(e) => toggleSelectProject(proj.id, e)}
                title="Chọn dự án này"
              />
              <h3 class="project-name">{proj.name}</h3>
            </div>

            <div class="card-top-actions">
              {#if proj.is_pinned}
                <span class="pin-badge" title="Được ghim">📌</span>
              {/if}
              <button 
                class="btn-delete-icon" 
                onclick={(e) => handleDeleteSingleProject(proj, e)}
                title="Xóa dự án này"
                aria-label="Xóa dự án"
              >
                🗑️
              </button>
            </div>
          </div>

          <div class="card-meta">
            <span>{proj.chapter_count || 1} chương · {proj.segment_count || 0} đoạn</span>
            <span class="model-tag">{proj.model}</span>
          </div>

          <!-- Progress Bar -->
          <div class="card-progress-section">
            <div class="progress-info">
              <span>{proj.completed_count || 0}/{proj.segment_count || 0} đoạn hoàn tất</span>
              <span>{Math.round(((proj.completed_count || 0) / (proj.segment_count || 1)) * 100)}%</span>
            </div>
            <div class="progress-bar-bg" role="progressbar" aria-valuenow={proj.completed_count || 0} aria-valuemax={proj.segment_count || 1}>
              <div 
                class="progress-bar-fill" 
                style={`width: ${Math.round(((proj.completed_count || 0) / (proj.segment_count || 1)) * 100)}%`}
              ></div>
            </div>
          </div>

          <div class="card-footer">
            <span class="updated-time">🕒 {new Date(proj.updated_at).toLocaleDateString('vi-VN')}</span>
            <span class="voice-badge">🎙️ {proj.voice}</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .project-library-container {
    padding: var(--space-6);
    height: 100%;
    overflow-y: auto;
    background: var(--color-bg-app);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .library-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .header-title h1 {
    font-size: var(--font-size-2xl);
    color: var(--color-text-primary);
  }

  .badge {
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
    font-weight: 600;
    font-size: var(--font-size-xs);
    padding: 4px 8px;
    border-radius: var(--radius-full);
  }

  .header-actions {
    display: flex;
    gap: var(--space-3);
  }

  .btn {
    height: var(--target-btn-md);
    padding: 0 var(--space-4);
    border-radius: var(--radius-md);
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: none;
    cursor: pointer;
  }

  .btn-primary {
    background: var(--color-accent);
    color: white;
  }

  .btn-secondary {
    background: var(--color-bg-surface-raised);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
  }

  .btn-danger {
    background: #dc2626;
    color: white;
    font-weight: 600;
  }
  .btn-danger:hover { background: #b91c1c; }

  .library-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .search-box {
    position: relative;
    width: 300px;
  }

  .search-icon {
    position: absolute;
    left: var(--space-3);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-muted);
  }

  .search-box input {
    width: 100%;
    padding-left: 36px;
    height: 36px;
    border-radius: 6px;
    border: 1px solid var(--color-border);
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .batch-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(220, 38, 38, 0.1);
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid rgba(220, 38, 38, 0.3);
  }

  .batch-count {
    font-size: 13px;
    color: var(--color-text-primary);
  }

  .select-all-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    cursor: pointer;
    color: var(--color-text-secondary);
  }

  .sort-box {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .sort-box select {
    height: 36px;
    padding: 0 10px;
    border-radius: 6px;
    border: 1px solid var(--color-border);
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
  }

  .projects-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }

  .project-card {
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
  }

  .project-card:hover {
    border-color: var(--color-accent);
    box-shadow: var(--shadow-md);
  }

  .project-card.selected {
    border-color: var(--color-accent);
    background: var(--color-bg-surface-selected);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .card-title-box {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
  }

  .card-checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .project-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--color-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 180px;
  }

  .card-top-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-delete-icon {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    opacity: 0.6;
    padding: 4px;
    border-radius: 4px;
    transition: opacity 0.2s, background 0.2s;
  }

  .btn-delete-icon:hover {
    opacity: 1;
    background: rgba(220, 38, 38, 0.15);
  }

  .card-meta {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .model-tag {
    background: var(--color-bg-surface-raised);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
  }

  .card-progress-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--color-text-secondary);
  }

  .progress-bar-bg {
    height: 6px;
    background: var(--color-bg-surface-raised);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.3s;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--color-text-muted);
    border-top: 1px solid var(--color-border);
    padding-top: 8px;
  }

  .empty-state {
    text-align: center;
    padding: 60px 20px;
    background: var(--color-bg-surface);
    border-radius: 12px;
    border: 1px dashed var(--color-border);
  }

  .empty-icon { font-size: 48px; margin-bottom: 12px; }
  .empty-actions { display: flex; justify-content: center; gap: 12px; margin-top: 16px; }
</style>
