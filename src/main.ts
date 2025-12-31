import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import router from './router';
import './styles/main.css';
import { useAppStore } from './stores';

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.use(router);

// 初始化应用设置（包括主题）
const appStore = useAppStore();
appStore.init();

app.mount('#app');
