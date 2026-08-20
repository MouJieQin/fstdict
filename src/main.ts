import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import i18n from '@/i18n'

import App from '@/App.vue'
import router from '@/router'
import '@/style.css'

const app = createApp(App)
const pinia = createPinia()

app.use(router)
app.use(ElementPlus)
app.use(pinia)
app.use(i18n)
app.mount('#app')