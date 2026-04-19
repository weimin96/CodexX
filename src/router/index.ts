import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/accounts',
    },
    {
      path: '/accounts',
      name: 'AccountList',
      component: () => import('@/views/AccountListView.vue'),
      meta: { title: '账号列表' },
    },
    {
      path: '/accounts/:id',
      name: 'AccountDetail',
      component: () => import('@/views/AccountDetailView.vue'),
      meta: { title: '账号详情' },
    },
    {
      path: '/usage',
      name: 'Usage',
      component: () => import('@/views/UsageView.vue'),
      meta: { title: '用量统计' },
    },
    {
      path: '/codex',
      name: 'CodexLaunch',
      component: () => import('@/views/CodexLaunchView.vue'),
      meta: { title: '启动器' },
    },
    {
      path: '/codex-config',
      name: 'CodexConfig',
      component: () => import('@/views/CodexConfigView.vue'),
      meta: { title: 'Codex 配置' },
    },
    {
      path: '/settings',
      name: 'Settings',
      component: () => import('@/views/SettingsView.vue'),
      meta: { title: '设置' },
    },
  ],
})

export default router
