import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        component: () => import('@/views/DictLayout.vue'),
        children: [
            {
                path: '',
                name: 'Home',
                component: () => import('@/views/Home.vue'),
            },
            {
                path: 'updater',
                name: 'Updater',
                component: () => import('@/views/Updater.vue'),
            },
            {
                path: 'dict/:id',
                name: 'Dict',
                component: () => import('@/views/DictPage.vue'),
                props: true,
            },
        ],
    },
]

const router = createRouter({
    history: createWebHashHistory(),
    routes,
})

// Redirect root path to default dictionary session
router.beforeEach((to) => {
    if (to.path === '/') {
        return '/dict/1'
    }
})

export default router