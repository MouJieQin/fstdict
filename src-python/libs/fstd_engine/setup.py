from setuptools import setup, Extension
import pybind11

# brew 安装的库 头文件 + 库路径（macOS 专用）
BREW_INCLUDE = "/usr/local/include"
BREW_LIB = "/usr/local/lib"
# 编译配置
ext_modules = [
    Extension(
        # 模块名：import fstd_engine
        name="fstd_engine",
        # 你的 C++ 源码文件名
        sources=["fstd_engine.cpp"],
        # 包含 pybind11 头文件
        include_dirs=[pybind11.get_include(), BREW_INCLUDE],
        library_dirs=[BREW_LIB],  # 关键：告诉链接器 fmt 库在哪里
        # C++ 标准
        language="c++",
        # 编译优化（生产环境必须开，速度拉满）
        extra_compile_args=["-O3", "-std=c++20", "-fvisibility=hidden"],
        extra_link_args=["-lfstd", "-lfmt"],
    ),
]

setup(
    name="fstd_engine",
    version="1.0",
    description="FSTD 词典单词存储 + 高速搜索引擎",
    ext_modules=ext_modules,
    zip_safe=False,
)
