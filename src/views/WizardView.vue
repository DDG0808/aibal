<script setup lang="ts">
// Phase 7: 首次设置向导
import { ref } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

const currentStep = ref(1);
const totalSteps = 3;

const nextStep = () => {
  if (currentStep.value < totalSteps) {
    currentStep.value++;
  }
};

const prevStep = () => {
  if (currentStep.value > 1) {
    currentStep.value--;
  }
};

const finish = async () => {
  // TODO: 保存配置
  const appWindow = getCurrentWindow();
  await appWindow.close();
};
</script>

<template>
  <div class="wizard-view">
    <header class="wizard-header">
      <h1>欢迎使用 CUK</h1>
      <p class="subtitle">
        让我们完成一些初始设置
      </p>
    </header>

    <main class="wizard-content">
      <!-- Step 1: 欢迎 -->
      <div
        v-if="currentStep === 1"
        class="wizard-step"
      >
        <div class="step-icon">
          <span>1</span>
        </div>
        <h2>准备开始</h2>
        <p>CUK 帮助你追踪 Claude AI 的使用量，让你随时了解使用情况。</p>
        <ul class="feature-list">
          <li>实时监控 Claude 使用量</li>
          <li>菜单栏快速访问</li>
          <li>使用量提醒通知</li>
        </ul>
      </div>

      <!-- Step 2: 配置 -->
      <div
        v-if="currentStep === 2"
        class="wizard-step"
      >
        <div class="step-icon">
          <span>2</span>
        </div>
        <h2>基本配置</h2>
        <p>配置你的偏好设置。</p>
        <div class="config-options">
          <label class="config-option">
            <input
              type="checkbox"
              checked
            >
            <span>开机自动启动</span>
          </label>
          <label class="config-option">
            <input
              type="checkbox"
              checked
            >
            <span>启用使用量提醒</span>
          </label>
        </div>
      </div>

      <!-- Step 3: 完成 -->
      <div
        v-if="currentStep === 3"
        class="wizard-step"
      >
        <div class="step-icon success">
          <span>✓</span>
        </div>
        <h2>设置完成</h2>
        <p>你已经准备好开始使用 CUK 了！</p>
        <p class="hint">
          点击菜单栏图标即可查看使用情况。
        </p>
      </div>
    </main>

    <!-- Progress -->
    <div class="wizard-progress">
      <div
        v-for="step in totalSteps"
        :key="step"
        class="progress-dot"
        :class="{ active: step === currentStep, completed: step < currentStep }"
      />
    </div>

    <footer class="wizard-footer">
      <button
        v-if="currentStep > 1"
        class="btn btn-secondary"
        @click="prevStep"
      >
        上一步
      </button>
      <button
        v-if="currentStep < totalSteps"
        class="btn btn-primary"
        @click="nextStep"
      >
        下一步
      </button>
      <button
        v-if="currentStep === totalSteps"
        class="btn btn-primary"
        @click="finish"
      >
        开始使用
      </button>
    </footer>
  </div>
</template>

<style scoped>
.wizard-view {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  padding: 32px;
  background: var(--bg-primary, #ffffff);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.wizard-header {
  text-align: center;
  margin-bottom: 32px;
}

.wizard-header h1 {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text-primary, #333);
}

.subtitle {
  margin: 8px 0 0;
  color: var(--text-secondary, #666);
}

.wizard-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.wizard-step {
  text-align: center;
  max-width: 400px;
}

.step-icon {
  width: 64px;
  height: 64px;
  margin: 0 auto 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 50%;
  color: white;
  font-size: 1.5rem;
  font-weight: bold;
}

.step-icon.success {
  background: linear-gradient(135deg, #34c759 0%, #30d158 100%);
}

.wizard-step h2 {
  margin: 0 0 12px;
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-primary, #333);
}

.wizard-step p {
  margin: 0 0 16px;
  color: var(--text-secondary, #666);
  line-height: 1.5;
}

.feature-list {
  list-style: none;
  padding: 0;
  margin: 0;
  text-align: left;
}

.feature-list li {
  padding: 8px 0;
  padding-left: 24px;
  position: relative;
  color: var(--text-primary, #333);
}

.feature-list li::before {
  content: "✓";
  position: absolute;
  left: 0;
  color: #34c759;
}

.config-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
  text-align: left;
}

.config-option {
  display: flex;
  align-items: center;
  gap: 12px;
  cursor: pointer;
}

.config-option input {
  width: 18px;
  height: 18px;
}

.hint {
  font-size: 0.875rem;
  color: var(--text-tertiary, #999);
}

.wizard-progress {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin: 32px 0;
}

.progress-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-color, #e5e5e5);
  transition: all 0.3s;
}

.progress-dot.active {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  transform: scale(1.25);
}

.progress-dot.completed {
  background: #34c759;
}

.wizard-footer {
  display: flex;
  justify-content: center;
  gap: 12px;
}

.btn {
  padding: 12px 24px;
  border: none;
  border-radius: 8px;
  font-size: 1rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.btn-primary:hover {
  opacity: 0.9;
  transform: translateY(-1px);
}

.btn-secondary {
  background: var(--bg-secondary, #f5f5f7);
  color: var(--text-primary, #333);
}

.btn-secondary:hover {
  background: var(--border-color, #e5e5e5);
}
</style>
