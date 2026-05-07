// .eslintrc.cjs
module.exports = {
  root: true,
  env: {
    node: true,
    browser: true,
    es2021: true
  },
  extends: [
    // 开启 Vue 语法检查
    "plugin:vue/vue3-essential",
    "eslint:recommended"
  ],
  parserOptions: {
    ecmaVersion: "latest",
    sourceType: "module"
  },
  rules: {
    // 可以自己开关警告/错误
    "no-unused-vars": "warn", // 未使用变量 → 警告
    "vue/no-mutating-props": "error", // 禁止修改 props → 报错
    "vue/valid-template-root": "error", // 模板必须有根节点
    "vue/no-v-html": "off" // 关闭 v-html 警告（需要就开）
  }
};