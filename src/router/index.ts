import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [

    {
        path: '/dict/:id',
        name: 'Dict',
        component: () => import('@/views/AppLayout.vue'),
        props: true,
    },

    {
        path: '/setting',
        name: 'Setting',
        component: () => import('@/views/Setting.vue'),
    },
    {
        path: '/manage',
        name: 'Manage',
        component: () => import('@/views/Manage.vue'),
    },
    {
        path: '/notification',
        name: 'Notification',
        component: () => import('@/views/Notification.vue'),
    },
    {
        path: '/updater',
        name: 'Updater',
        component: () => import('@/views/Updater.vue'),
    },
    {
        path: '/permission',
        name: 'Permission',
        component: () => import('@/views/Permission.vue'),
    },
]

const router = createRouter({
    history: createWebHashHistory(),
    routes,
})

// Redirect root to default dict page
router.beforeEach((to) => {
    if (to.path === '/') {
        return '/dict/1'
    }
    return true
})

export default router
