import { createApp } from 'vue';
import App from '@/App.vue';
import MiniView from '@/components/MiniView.vue';

import '@/styles/theme.scss';
import '@/styles/main.scss';

createApp(location.hash === '#mini' ? MiniView : App).mount('#app');
