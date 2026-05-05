<template>
  <div ref="listRef" class="dict-select-sort-dialog">
    <div class="dict-settings-drag-cards" v-for="item in list" :key="item.id">
      <el-card class="dict-settings-drag-card" shadow="always" :class="{ 'is-disabled': !item.is_enabled }">
        <div class="dict-settings-drag-card-content">
          <div class="left-group">
            <el-image :src="item.cover_url" class="icon">
              <template #error>
                <BiSolidBookBookmark size="35" />
              </template>
            </el-image>
            <span class="name">{{ item.name }}</span>
          </div>
          <div class="right-group">
            <el-switch v-model="item.is_enabled" />
          </div>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, nextTick } from 'vue'
import Sortable from 'sortablejs'

import type { DictSettingInfo, DictsSettingInfo, SessionConfig } from '@/common/type-interface'
import type { PropType } from 'vue'
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { SessionWebSocketService } from '@/common/session-websocket-client'

const props = defineProps({
  dictSSDialogVisible: {
    type: Boolean,
    required: true // 修复拼写错误：require → required
  },
  webSocket: {
    type: [SessionWebSocketService, null],
    required: true
  },
  sessionConfig: {
    type: Object as PropType<SessionConfig>,
    required: true
  }
})

const listRef = ref<HTMLElement | null>(null)
const sessionConfig = ref<SessionConfig>(JSON.parse(JSON.stringify(props.sessionConfig || {})))
// 声明数组类型，兼容 Tauri 响应式
const list = ref<DictSettingInfo>(sessionConfig.value?.dictsSettingInfo || [])

// 拖拽实例（方便销毁，避免内存泄漏）
let sortableInstance: Sortable | null = null

// 初始化拖拽（核心：nextTick 等待 DOM 渲染完成）
const initSortable = () => {
  // 销毁旧实例
  if (sortableInstance) {
    sortableInstance.destroy()
    sortableInstance = null
  }

  if (!listRef.value) return

  // 关键：Tauri 必须加 draggable 选择器
  sortableInstance = Sortable.create(listRef.value, {
    animation: 300,
    draggable: '.dict-settings-drag-cards', // 指定可拖拽元素
    ghostClass: 'sortable-ghost',
    onEnd: ({ oldIndex, newIndex }) => {
      // 安全判断索引
      if (oldIndex === undefined || newIndex === undefined) return
      if (oldIndex === newIndex) return

      const item = list.value?.splice(oldIndex as number, 1)[0]
      list.value?.splice(newIndex as number, 0, item as DictSettingInfo)
      console.log('New list order:', list.value?.map(i => i.name).join(', '))
    }
  })
}

// 弹窗打开时初始化拖拽
watch(() => props.dictSSDialogVisible, async (newVal) => {
  if (newVal) {
    // 深拷贝数据
    sessionConfig.value = JSON.parse(JSON.stringify(props.sessionConfig || {}))
    list.value = sessionConfig.value?.dictsSettingInfo || []

    // 等待 DOM 渲染完成后初始化拖拽
    await nextTick()
    initSortable()
  } else {
    // 关闭弹窗时保存数据
    if (JSON.stringify(sessionConfig.value) !== JSON.stringify(props.sessionConfig)) {
      props.webSocket?.sendSessionConfig(sessionConfig.value)
    }
  }
}, { deep: true })

// 页面挂载（兜底初始化）
onMounted(() => {
  if (props.dictSSDialogVisible) {
    nextTick(() => initSortable())
  }
})
</script>

<style scoped>
/* 拖拽占位样式（可选，优化体验） */
:deep(.sortable-ghost) {
  opacity: 0.5;
  background: #f5f5f5;
}
</style>