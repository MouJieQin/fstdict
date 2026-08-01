<template>
  <div v-if="isTauriEnv" class="drag-area" :class="{ active: dragOver }">
    <BsUpload size="35" />
    <div>
      拖拽(.fstdx .fstdd)或(.mdx .mdd)文件到此
    </div>
    <div>
      您可以访问论坛 <el-link href="https://forum.freemdict.com" type="primary" target="_blank">freemdict</el-link>，或直接从
      <el-link href="https://downloads.freemdict.com" type="primary" target="_blank">download</el-link> 下载词典资源
    </div>
  </div>
  <div class="dict-set-options">
    <div class="dict-set-options-control">
      <div style="text-align: center;">
        <el-button type="primary" :icon="Plus" @click="handleCreateDictSetOption"></el-button>
        <el-button type="danger" :icon="Delete" @click="handleDeleteSelected"
          :disabled="disableDeleteButton"></el-button>
        <el-button :icon="Edit" @click="handleRenameDictSetOption" :disabled="disableEditButton"></el-button>

        <el-select v-model="localSessionConfig.dict_setting_option_name" filterable
          placeholder="Select dict settings option" style="margin-left: 20px;max-width: 240px;">
          <el-option v-for="(_, name) in localSystemConfig.dict_set_options" :key="name" :label="name" :value="name" />
        </el-select>
      </div>
    </div>
    <div ref="listRef" class="dict-select-sort-dialog">
      <div class="dict-settings-drag-cards" v-for="item in list" :key="item.name">
        <el-card class="dict-settings-drag-card" shadow="always" :class="{ 'is-disabled': !item.is_enabled }">
          <div class="dict-settings-drag-card-content">
            <div class="left-group">
              <el-image :src="props.dictsInfo[item.name]?.cover_url" class="icon">
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
  </div>
  <el-dialog class="add-dict-dialog" v-model="addDictVisible" :title="`Adding Dictionary`" width="700"
    :close-on-click-modal="false" :close-on-press-escape="false" draggable :show-close="false">
    <!-- :close-on-click-modal="false" :close-on-press-escape="false" draggable :show-close="false"> -->
    <div v-for="(item, index) in props.addDictMsgs" :key="index">
      <p v-if="item.type == 'info'" style="color:var(--el-color-primary)">{{ item.msg }}</p>
      <p v-else-if="item.type == 'warning'" style="color:var(--el-color-warning)">{{ item.msg }}</p>
      <p v-else-if="item.type == 'error'" style="color:var(--el-color-danger)">{{ item.msg }}</p>
      <p v-else-if="item.type == 'success'" style="color:var(--el-color-success)">{{ item.msg }}</p>
      <p v-else-if="item.type == 'done'" style="color:var(--el-color-success)">Done.</p>
    </div>
    <template #footer>
      <div class="dialog-footer">
        <el-button v-if="isAddDictDone" type="primary" @click="addDictVisible = false">
          Confirm
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, onBeforeUnmount, watch, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import Sortable from 'sortablejs'
import { MoreFilled, Delete, Edit, Plus } from '@element-plus/icons-vue'
import type { DictSettingInfo, DictsSettingInfo, SessionConfig, DictInfo } from '@/common/type-interface'
import { useSystemConfigStore } from '@/stores/stores'
import type { PropType } from 'vue'
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElNotification } from 'element-plus'
import { BsUpload } from 'vue-icons-plus/bs'

const addDictVisible = ref(false)

const isAddDictDone = computed(() => {
  return props.addDictMsgs.length == 0 ? false : props.addDictMsgs[props.addDictMsgs.length - 1].type == 'done'
})

const isTauriEnv = computed(() => {
  return props.env === ''
})

const disableDeleteButton = computed(() => {
  return localSessionConfig.value.dict_setting_option_name === 'default'
})

const disableEditButton = computed(() => {
  return localSessionConfig.value.dict_setting_option_name === 'default'
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
  dictsInfo: {
    type: Object as PropType<DictsSettingInfo>,
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

const emits = defineEmits<{
  (e: 'clear:addDictMsgs'): void
}>()

const listRef = ref<HTMLElement | null>(null)
const localSessionConfig = ref<SessionConfig>(JSON.parse(JSON.stringify(props.sessionConfig || {})))
const systemConfigStore = useSystemConfigStore();
const localSystemConfig = ref<any>(JSON.parse(JSON.stringify(systemConfigStore.systemConfig)))
const list = ref<DictSettingInfo>(localSystemConfig.value?.dict_set_options[localSessionConfig.value?.dict_setting_option_name] || [])

watch(() => systemConfigStore.systemConfig, (newVal) => {
  localSystemConfig.value = JSON.parse(JSON.stringify(newVal))
}, { deep: true })

watch(() => localSessionConfig.value.dict_setting_option_name, async (name) => {
  update_system_config_if_need()
  list.value = localSystemConfig.value?.dict_set_options[name] || []
  await nextTick()
  initSortable()  // 重新初始化拖拽
})

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
    fallbackOnBody: false,

    onEnd: ({ oldIndex, newIndex }) => {
      if (oldIndex === undefined || newIndex === undefined) return
      if (oldIndex === newIndex) return

      const item = list.value.splice(oldIndex as number, 1)[0]
      list.value.splice(newIndex as number, 0, item)

      // 同步回配置对象
      const name = localSessionConfig.value.dict_setting_option_name
      if (name && localSystemConfig.value?.dict_set_options?.[name]) {
        localSystemConfig.value.dict_set_options[name] = [...list.value]
      }
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
      if (value.length > 30) {
        return "不能超过30个字符"
      }
    }
  })
    .then(({ value }) => {
      update_system_config_if_need()
      props.webSocket?.sendCreateDictSetOption(value)
      localSessionConfig.value.dict_setting_option_name = value
      update_session_config_if_need()
    })
    .catch(() => {
    })
}

const handleRenameDictSetOption = () => {
  ElMessageBox.prompt('请重命名词典设置可选项的名字', 'Tip', {
    confirmButtonText: 'OK',
    cancelButtonText: 'Cancel',
    inputValue: localSessionConfig.value.dict_setting_option_name,
    inputValidator: (value: string) => {
      const dict_set_options: {} = localSystemConfig.value?.dict_set_options;
      if (value in dict_set_options) {
        return "该名字已存在"
      }
      if (value.length > 30) {
        return "不能超过30个字符"
      }
    }
  })
    .then(({ value }) => {
      update_system_config_if_need()
      props.webSocket?.sendRenameDictSetOption(localSessionConfig.value.dict_setting_option_name, value)
      localSessionConfig.value.dict_setting_option_name = value
      update_session_config_if_need()
    })
    .catch(() => {
    })
}

const handleDeleteSelected = () => {
  ElMessageBox.confirm(
    `The dictioary setting options 「${localSessionConfig.value.dict_setting_option_name}」 will be deleted. Continue?`,
    'Warning',
    {
      confirmButtonText: 'OK',
      cancelButtonText: 'Cancel',
      type: 'warning',
      center: true,
    }
  )
    .then(() => {
      props.webSocket?.sendRemoveDictSetOption(localSessionConfig.value.dict_setting_option_name)
      localSessionConfig.value.dict_setting_option_name = 'default'
    })
    .catch(() => {
    })
}

const refresh_dict_info = async () => {
  // 深拷贝数据
  localSessionConfig.value = JSON.parse(JSON.stringify(props.sessionConfig || {}))
  list.value = localSystemConfig.value?.dict_set_options[localSessionConfig.value?.dict_setting_option_name] || []

  // 等待 DOM 渲染完成后初始化拖拽
  await nextTick()
  initSortable()
}

// watch(() => props.addDictMsgs, async (newVal) => {
//   if (newVal.length > 0) {
//     let msg = ''
//     for (let item of newVal) {
//       msg += item.msg + '\n'
//     }
//     ElNotification({
//       title: 'Prompt',
//       message: msg,
//       duration: 0,
//     })
//   }
// })

watch(() => props.refreshDicsSettingsInfoFlag, async (newVal) => {
  await refresh_dict_info()
})

const update_system_config_if_need = () => {
  if (JSON.stringify(localSystemConfig.value) !== JSON.stringify(systemConfigStore.systemConfig)) {
    props.webSocket.sendUpdateSystemConfig(localSystemConfig.value)
  }
}

const update_session_config_if_need = () => {
  if (JSON.stringify(localSessionConfig.value) !== JSON.stringify(props.sessionConfig)) {
    props.webSocket.sendSessionConfig(localSessionConfig.value)
  }
}

// 弹窗打开时初始化拖拽
watch(() => props.dictSSDialogVisible, async (newVal) => {
  if (newVal) {
    await refresh_dict_info()
  } else {
    // 关闭弹窗时保存数据
    update_system_config_if_need()
    update_session_config_if_need()
  }
}, { deep: true })

// 页面挂载（兜底初始化）
onMounted(async () => {
  if (props.dictSSDialogVisible) {
    nextTick(() => initSortable())
  }

  if (props.env === '') {
    // Get a handle on the current application window context
    const webview = getCurrentWebview()

    // Subscribe directly to the OS-level system file-dropping stream
    unlistenDragDrop = await webview.onDragDropEvent((event) => {
      switch (event.payload.type) {
        case 'enter':
        case 'over':
          // Triggers when files break the visual application viewport barrier
          // 判断鼠标位置是否在拖拽区域内
          const dragArea = document.querySelector('.drag-area')?.getBoundingClientRect()
          if (dragArea) {
            const { x, y } = event.payload.position || { x: 0, y: 0 }
            dragOver.value = x >= dragArea.left && x <= dragArea.right && y >= dragArea.top && y <= dragArea.bottom
          }
          break
        case 'drop':
          // Triggers once the user releases the files onto the app window boundary
          if (dragOver.value) {
            // event.payload.paths contains the complete array of true absolute file string paths
            const absolutePaths = event.payload.paths
            console.log('Absolute system paths extracted:', absolutePaths)
            handleFileProcessing(absolutePaths)
            dragOver.value = false
          }
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
  }
})

// Always clean up window level global background listeners to prevent memory leaks
onBeforeUnmount(() => {
  if (unlistenDragDrop) {
    unlistenDragDrop()
  }
})

const handleFileProcessing = async (paths: string[]) => {
  emits('clear:addDictMsgs')
  addDictVisible.value = true
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