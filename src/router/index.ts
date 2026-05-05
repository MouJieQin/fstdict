import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

// 懒加载组件
const Dict = () => import('@/views/DictPage.vue')
const Home = () => import('@/views/Home.vue')

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        component: () => import('@/views/DictLayout.vue'),
        children: [
            {
                path: '',
                name: 'Home',
                component: Home
            },
            {
                path: 'dict/:id',
                name: 'Dict',
                component: Dict
            }
        ]
    },
]

const router = createRouter({
    history: createWebHashHistory(),
    routes
})

router.beforeEach((to) => {
    // 访问根路径 / 时，重定向到 /dict/1
    if (to.path === '/') {
        return '/dict/1' // 直接返回路径，替代 next('/dict/1')
    }
    // 其他情况直接放行（无需 return，等同于 next()）
})

export default router
