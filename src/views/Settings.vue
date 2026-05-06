<template>
    <div class="setting-container">
        <div>
            <p class="system-config-title">系统设置</p>
        </div>
        <div>
            <div class="config-class">
                <p class="config-class-title">收藏夹列表</p>
                <el-table v-if="localSystemConfig" :data="localSystemConfig.folders.folder_info" height="350"
                    style="width: 100%" @selection-change="handleSelectionChange" stripe>
                    <el-table-column type="selection" width="55" />
                    <el-table-column fixed prop="name" label="Name" width="130" show-overflow-tooltip sortable />
                    <el-table-column prop="words_count" label="Total" sortable />
                    <el-table-column prop="created_at" label="Created" width="110" show-overflow-tooltip sortable />
                    <el-table-column prop="description" label="Description" width="180" show-overflow-tooltip />
                    <el-table-column fixed="right" label="Operations" width="130">
                        <template #default="scope">
                            <el-button-group>
                                <el-button :icon="Edit" size="small" @click="handleEdit(scope.$index, scope.row)" />
                                <el-button :icon="Document" size="small" @click="handleView(scope.$index, scope.row)" />
                                <el-popconfirm confirm-button-text="删除" confirm-button-type="danger"
                                    cancel-button-text="取消" :icon="Delete" icon-color="#FF4949" title="确定删除收藏夹吗？"
                                    @confirm="handleDelete(scope.$index, scope.row)">
                                    <template #reference>
                                        <el-button :icon="Delete" size="small" type="danger" />
                                    </template>
                                </el-popconfirm>
                            </el-button-group>
                        </template>
                    </el-table-column>
                </el-table>
                <div style="margin-top: 20px">
                    <el-button type="primary" :icon="Plus" @click="handleCreateFolder"></el-button>
                    <el-button type="danger" :icon="Delete" @click="deleteDialogVisible = true"
                        :disabled="disableDeleteButton"></el-button>
                    <el-button @click="handleUpdateToAnki" :disabled="disableAnkiButton">
                        <AnkiIcon :size="24" style="margin-right: 8px;" />
                        Update to Anki
                    </el-button>
                    <el-select v-if="localSystemConfig" v-model="localSessionConfig.default_folder.id" filterable
                        placeholder="Select Default Folder" style="margin-left: 20px;max-width: 240px">
                        <el-option v-for="item in defaultFolderOptions" :key="item.id" :label="item.name"
                            :value="item.id" />
                    </el-select>
                </div>
            </div>
        </div>

        <el-dialog v-model="createOrEditDialogVisible" :title="dialogTitle" width="500" align-center>
            <el-form ref="ruleFormRef" style="max-width: 600px" :model="ruleForm" status-icon :rules="rules"
                label-width="auto" class="demo-ruleForm">
                <el-form-item label="Name" prop="name" required>
                    <el-input v-model="ruleForm.name" autocomplete="off" />
                </el-form-item>
                <el-form-item label="Description" prop="description">
                    <el-input v-model="ruleForm.description" autocomplete="off" type="textarea" />
                </el-form-item>
            </el-form>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="createOrEditDialogVisible = false">Cancel</el-button>
                    <el-button type="primary" @click="submitForm(ruleFormRef)">
                        Confirm
                    </el-button>
                </div>
            </template>
        </el-dialog>

        <el-dialog v-model="deleteDialogVisible" :title="`Delete ${multipleSelection.length} selected folders?!!!`"
            width="500" align-center>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="deleteDialogVisible = false">Cancel</el-button>
                    <el-button type="danger" @click="handleDeleteSelected">
                        Confirm
                    </el-button>
                </div>
            </template>
        </el-dialog>

        <el-dialog v-model="favoriteWordsDialogVisible" fullscreen :z-index="10000">
            <FavoriteWords :favoriteWordsDialogVisible="favoriteWordsDialogVisible" :webSocket="props.webSocket"
                @update-visible="(visible) => favoriteWordsDialogVisible = visible" :favoriteWords="favoriteWords"
                :folderName="folderIdNameToShow" :folderId="folderIdToShow" />
        </el-dialog>
    </div>
</template>


<script lang="ts" setup>
import { reactive, ref, onBeforeMount, watch, computed } from 'vue'
import type { PropType } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { useSystemConfigStore } from '@/stores/stores'
import { Edit, Delete, Document, Plus } from '@element-plus/icons-vue'

import AnkiIcon from '@/components/Icons/AnkiIcon.vue'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, SystemConfig, FolderInfo, FolderWords, WordInfoWithFavoriteAt } from '@/common/type-interface'
import FavoriteWords from '@/components/Dialogs/FavoriteWords.vue'


const props = defineProps({
    settingDialogVisible: {
        type: Boolean,
        require: true
    },
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true
    },
    sessionConfig: {
        type: Object as PropType<SessionConfig>,
        required: true
    },
    folderWords: {
        type: Object as () => FolderWords,
        required: true,
        default: () => ({}),
    },
})

const systemConfigStore = useSystemConfigStore()
const localSystemConfig = ref<SystemConfig>(JSON.parse(JSON.stringify(systemConfigStore.systemConfig)))
const localSessionConfig = ref<SessionConfig>(JSON.parse(JSON.stringify(props.sessionConfig)))
const multipleSelection = ref<FolderInfo[]>([])
const createOrEditDialogVisible = ref(false)
const deleteDialogVisible = ref(false)
const favoriteWordsDialogVisible = ref(false)
const isCreate = ref(true)
const folderIdEditing = ref('')
const folderIdToShow = ref<number>(0)
const folderIdNameToShow = ref('')

watch(() => systemConfigStore.systemConfig, (newVal) => {
    localSystemConfig.value = JSON.parse(JSON.stringify(newVal))
})

watch(() => localSessionConfig.value.default_folder.id, () => {
    props.webSocket?.sendSessionConfig(localSessionConfig.value)
})

const dialogTitle = computed(() => {
    return isCreate.value ? 'Create New Folder' : 'Edit Folder'
})

const defaultFolderOptions = computed(() => {
    return localSystemConfig.value.folders.folder_info.map((item) => ({
        id: item.id,
        name: item.name,
    }))
})

const disableDeleteButton = computed(() => {
    return multipleSelection.value.length === 0
})

const disableAnkiButton = computed(() => {
    return multipleSelection.value.length === 0
})

const favoriteWords = computed(() => {
    return props.folderWords[folderIdToShow.value] || []
})

onBeforeMount(() => {
    props.webSocket?.sendSystemConfig()
})

const ruleFormRef = ref<FormInstance>()

const validateName = (_: any, value: any, callback: any) => {
    if (value === '') {
        callback(new Error('Please input the name'))
    } else if (value.length > 20) {
        callback(new Error('Name must be less than 20 characters'))
    } else if (localSystemConfig.value.folders.folder_info.some((item) => item.name === value)) {
        if (isCreate.value) {
            callback(new Error('Name already exist'))
        }
        else {
            callback()
        }
    }
    else {
        callback()
    }
}
const validateDescription = (_: any, __: any, callback: any) => {
    callback()
}

const ruleForm = reactive({
    name: '',
    description: '',
})

const rules = reactive<FormRules<typeof ruleForm>>({
    name: [{ validator: validateName, trigger: 'blur' }],
    description: [{ validator: validateDescription, trigger: 'blur' }],
})

const submitForm = (formEl: FormInstance | undefined) => {
    if (!formEl) return
    formEl.validate((valid) => {
        if (valid) {
            if (isCreate.value) {
                props.webSocket?.sendCreateFolder(
                    ruleForm.name,
                    ruleForm.description,
                )
            } else {
                props.webSocket?.sendUpdateFolder(
                    Number(folderIdEditing.value),
                    ruleForm.name,
                    ruleForm.description,
                )
            }
            createOrEditDialogVisible.value = false
        } else {
            console.log('error submit!')
        }
    })
}


const handleCreateFolder = () => {
    isCreate.value = true
    createOrEditDialogVisible.value = true
}

const handleEdit = (_: number, row: any) => {
    isCreate.value = false
    ruleForm.name = row.name
    ruleForm.description = row.description
    createOrEditDialogVisible.value = true
    folderIdEditing.value = row.id
}

const handleView = (_: number, row: any) => {
    folderIdToShow.value = row.id
    folderIdNameToShow.value = row.name
    props.webSocket?.sendFavoriteWordsRequest(row.id)
    favoriteWordsDialogVisible.value = true
}

const handleDelete = (_: number, row: any) => {
    props.webSocket?.sendDeleteFolder(row.id)
}

const handleDeleteSelected = () => {
    multipleSelection.value.forEach((item: FolderInfo) => {
        props.webSocket?.sendDeleteFolder(item.id)
    })
    deleteDialogVisible.value = false
    multipleSelection.value = []
}

const handleUpdateToAnki = () => {
    multipleSelection.value.forEach((item: FolderInfo) => {
        props.webSocket?.sendUpdateToAnki(item.name, item.id)
    })
}

const handleSelectionChange = (val: FolderInfo[]) => {
    multipleSelection.value = val
}

</script>
