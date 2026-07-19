<template>
  <div v-if="isTauriEnv" class="drag-area" :class="{ active: dragOver }">
    <BsUpload size="35" />
    拖拽(.fstdx .fstdd)或(.mdx .mdd)文件到此
  </div>
  <div class="dict-set-options-control">
    <div style="text-align: center;">
      <el-button type="primary" :icon="Plus" @click="handleCreateDictSetOption"></el-button>
      <el-button type="danger" :icon="Delete" @click="handleDeleteSelected" :disabled="disableDeleteButton"></el-button>
      <el-select v-model="localSessionConfig.dictsSettingInfoName" filterable placeholder="Select dict settings option"
        style="margin-left: 20px;max-width: 240px;">
        <el-option v-for="(_, name) in localSystemConfig.dict_set_options" :key="name" :label="name" :value="name" />
      </el-select>
    </div>
  </div>
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
            <el-switch v-model="item.is_enabled" style="margin-right: 30px;" />
            <el-dropdown placement="bottom-end" @command="handleDropdownCommand">
              <el-icon style="align-items: center;">
                <MoreFilled />
              </el-icon>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item :command="{ cmd: 'showInFolder', name: item.name }">在文件夹中显示</el-dropdown-item>
                  <el-dropdown-item :command="{ cmd: 'delete', name: item.name }">
                    <el-icon>
                      <Delete style="color: #FF4949;" />
                    </el-icon>
                    <span>删除</span>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, onBeforeUnmount, watch, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import Sortable from 'sortablejs'
import { MoreFilled, Delete, Edit, Plus } from '@element-plus/icons-vue'
import type { DictSettingInfo, DictsSettingInfo, SessionConfig } from '@/common/type-interface'
import { useSystemConfigStore } from '@/stores/stores'
import type { PropType } from 'vue'
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElNotification } from 'element-plus'
import { BsUpload } from 'vue-icons-plus/bs'


const isTauriEnv = computed(() => {
  return props.env === ''
})

const dragOver = ref(false)
let unlistenDragDrop: (() => void) | null = null

const props = defineProps({
  dictSSDialogVisible: {
    type: Boolean,
    required: true // 修复拼写错误：require → required
  },
  env: {
    type: String,
    default: 'web'
  },
  webSocket: {
    type: [SessionWebSocketService, null],
    required: true
  },
  sessionConfig: {
    type: Object as PropType<SessionConfig>,
    required: true
  },
  addDictMsgs: {
    type: Array,
    default: () => [],
  },
  refreshDicsSettingsInfoFlag: {
    type: Boolean,
    default: true,
  },
})

const listRef = ref<HTMLElement | null>(null)
const localSessionConfig = ref<SessionConfig>(JSON.parse(JSON.stringify(props.sessionConfig || {})))
const systemConfigStore = useSystemConfigStore();
const localSystemConfig = ref<any>(JSON.parse(JSON.stringify(systemConfigStore.systemConfig)))

watch(() => systemConfigStore.systemConfig, (newVal) => {
  localSystemConfig.value = JSON.parse(JSON.stringify(newVal))
}, { deep: true })

// 声明数组类型，兼容 Tauri 响应式
const list = ref<DictSettingInfo>(localSystemConfig.value?.dict_set_options[localSessionConfig.value?.dictsSettingInfoName] || [])

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
    draggable: '.dict-settings-drag-cards',
    ghostClass: 'sortable-ghost',

    // 关键：强制使用鼠标事件模拟拖拽，绕过 HTML5 DnD API
    forceFallback: true,

    // 可选：调整 fallback 体验
    fallbackClass: 'sortable-dragging',
    fallbackOnBody: true,

    onEnd: ({ oldIndex, newIndex }) => {
      if (oldIndex === undefined || newIndex === undefined) return
      if (oldIndex === newIndex) return

      const item = list.value?.splice(oldIndex as number, 1)[0]
      list.value?.splice(newIndex as number, 0, item as DictSettingInfo)
    }
  })
}

// 删除字典
const handleDeleteDict = (name: string) => {
  ElMessageBox.confirm(
    `Are you sure you want to delete the dict ${name}? This will permanently delete the dict and all its data.`,
    'Warning',
    {
      confirmButtonText: 'OK',
      cancelButtonText: 'Cancel',
      type: 'warning',
      center: true,
    }
  )
    .then(() => {
      props.webSocket?.sendDeleteDict(name)
    })
    .catch(() => {
      ElMessage({
        type: 'info',
        message: 'Delete canceled',
      })
    })
}

const handleDropdownCommand = (command: { cmd: string, name: string }) => {
  if (command.cmd === 'showInFolder') {
    props.webSocket?.sendShowDictInFolder(command.name)
  } else if (command.cmd === 'delete') {
    handleDeleteDict(command.name)
  }
}

const handleCreateDictSetOption = () => {
  ElMessageBox.prompt('请输入新的词典设置可选项的名字', 'Tip', {
    confirmButtonText: 'OK',
    cancelButtonText: 'Cancel',
    inputValidator: (value: string) => {
      const dict_set_options: {} = localSystemConfig.value?.dict_set_options;
      if (value in dict_set_options) {
        return "该名字已存在"
      }
    }
  })
    .then(({ value }) => {
      props.webSocket?.sendCreateDictSetOption(value)
      localSessionConfig.value.dictsSettingInfoName = value
    })
    .catch(() => {
    })
}

const refresh_dict_info = async () => {
  // 深拷贝数据
  localSessionConfig.value = JSON.parse(JSON.stringify(props.sessionConfig || {}))
  list.value = localSystemConfig.value?.dict_set_options[localSessionConfig.value?.dictsSettingInfoName] || []

  // 等待 DOM 渲染完成后初始化拖拽
  await nextTick()
  initSortable()
}

watch(() => props.addDictMsgs, async (newVal) => {
  if (newVal.length > 0) {
    let msg = ''
    for (let item of newVal) {
      msg += item.msg + '\n'
    }
    ElNotification({
      title: 'Prompt',
      message: msg,
      duration: 0,
    })
  }
})

watch(() => props.refreshDicsSettingsInfoFlag, async (newVal) => {
  await refresh_dict_info()
})

// 弹窗打开时初始化拖拽
watch(() => props.dictSSDialogVisible, async (newVal) => {
  if (newVal) {
    await refresh_dict_info()
  } else {
    // 关闭弹窗时保存数据
    if (JSON.stringify(localSessionConfig.value) !== JSON.stringify(props.sessionConfig)) {
      props.webSocket?.sendSessionConfig(localSessionConfig.value)
    }
  }
}, { deep: true })

// 页面挂载（兜底初始化）
onMounted(async () => {
  if (props.dictSSDialogVisible) {
    nextTick(() => initSortable())
  }
  // Get a handle on the current application window context
  const webview = getCurrentWebview()

  // Subscribe directly to the OS-level system file-dropping stream
  unlistenDragDrop = await webview.onDragDropEvent((event) => {
    switch (event.payload.type) {
      case 'enter':
      case 'over':
        // Triggers when files break the visual application viewport barrier
        dragOver.value = true
        break

      case 'drop':
        // Triggers once the user releases the files onto the app window boundary
        dragOver.value = false

        // event.payload.paths contains the complete array of true absolute file string paths
        const absolutePaths = event.payload.paths
        console.log('Absolute system paths extracted:', absolutePaths)

        // Process your collected absolute paths safely
        handleFileProcessing(absolutePaths)
        break

      case 'cancel':
        // Triggers if the user drags out of the app window without releasing the cursor
        dragOver.value = false
        break

      case 'leave':
      default:
        // Triggers if a user exits the view boundary without letting go of the cursor
        dragOver.value = false
        break
    }
  })
})

// Always clean up window level global background listeners to prevent memory leaks
onBeforeUnmount(() => {
  if (unlistenDragDrop) {
    unlistenDragDrop()
  }
})

const handleFileProcessing = async (paths: string[]) => {
  for (const filePath of paths) {
    props.webSocket?.sendAddDictionary(filePath)
  }
}

</script>

<style scoped>
/* 拖拽占位样式（可选，优化体验） */
:deep(.sortable-ghost) {
  opacity: 0.5;
  background: #f5f5f5;
}

.drag-area {
  /* width: 420px; */
  border-radius: 12px;
  height: 100px;
  margin: 0 auto;
  max-width: 960px;
  border: 2px dashed #ccc;
  display: grid;
  margin-bottom: 12px;
  place-items: center;
}

.active {
  border-color: #2584ff;
  background: #e8f3ff;
}
</style>