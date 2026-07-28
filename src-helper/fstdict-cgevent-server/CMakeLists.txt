cmake_minimum_required (VERSION 3.24)

# 全局策略
cmake_policy(SET CMP0148 NEW)

# ===================== Build Type Postfix Config =====================
set(RELEASE_POSTFIX "" CACHE STRING "Postfix for release build binaries")
set(DEBUG_POSTFIX "d" CACHE STRING "Postfix for debug build binaries")
set(COVERAGE_POSTFIX "" CACHE STRING "Postfix for coverage build binaries")
set(GPERF_POSTFIX "" CACHE STRING "Postfix for gperf build binaries")

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE "Release" CACHE STRING "Choose the type of build." FORCE)
endif()

string(TOLOWER "${CMAKE_BUILD_TYPE}" CMAKE_BUILD_TYPE_LOWER)

if (CMAKE_BUILD_TYPE_LOWER STREQUAL "release")
    set(BUILD_TYPE ${RELEASE_POSTFIX})
elseif (CMAKE_BUILD_TYPE_LOWER STREQUAL "debug")
    set(BUILD_TYPE ${DEBUG_POSTFIX})
elseif(CMAKE_BUILD_TYPE_LOWER STREQUAL "coverage")
    set(BUILD_TYPE ${COVERAGE_POSTFIX})
elseif(CMAKE_BUILD_TYPE_LOWER STREQUAL "gperf")
    set(BUILD_TYPE ${GPERF_POSTFIX})
else()
    message(WARNING "Unknown build type '${CMAKE_BUILD_TYPE}', defaulting to Release")
    set(BUILD_TYPE ${RELEASE_POSTFIX})
    set(CMAKE_BUILD_TYPE "Release" CACHE STRING "Build type" FORCE)
    string(TOLOWER "${CMAKE_BUILD_TYPE}" CMAKE_BUILD_TYPE_LOWER)
endif()

# ===================== Project Basic =====================
project(fstdict_cgevent_server LANGUAGES CXX)
set(PROJECT_ROOT ${CMAKE_CURRENT_SOURCE_DIR})

# C++20 strict
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Binary output directory
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

# ===================== Source Files =====================
set(SRC_MAIN
    ${PROJECT_ROOT}/main.cpp
    ${PROJECT_ROOT}/src/CGEventHandler.cpp
)
aux_source_directory(${PROJECT_ROOT}/src/libs DIR_LIB_SRCS)

add_executable(fstdict_cgevent_server${BUILD_TYPE}
    ${SRC_MAIN}
    ${DIR_LIB_SRCS}
)

# Header search path (modern target-scoped)
target_include_directories(fstdict_cgevent_server${BUILD_TYPE}
    PRIVATE
        ${PROJECT_ROOT}/src
)

# ===================== Compile Options =====================
set(COMMON_COMPILE_OPTIONS
    -W
    -Wall
)

if (CMAKE_BUILD_TYPE_LOWER STREQUAL "release")
    target_compile_options(fstdict_cgevent_server${BUILD_TYPE} PRIVATE
        ${COMMON_COMPILE_OPTIONS}
        -O2
    )
elseif (CMAKE_BUILD_TYPE_LOWER STREQUAL "debug")
    target_compile_options(fstdict_cgevent_server${BUILD_TYPE} PRIVATE
        ${COMMON_COMPILE_OPTIONS}
        -O0
        -g
        -DDEBUG
    )
elseif (CMAKE_BUILD_TYPE_LOWER STREQUAL "coverage")
    target_compile_options(fstdict_cgevent_server${BUILD_TYPE} PRIVATE
        ${COMMON_COMPILE_OPTIONS}
        -O0
        -g
        --coverage
    )
elseif (CMAKE_BUILD_TYPE_LOWER STREQUAL "gperf")
    target_compile_options(fstdict_cgevent_server${BUILD_TYPE} PRIVATE
        ${COMMON_COMPILE_OPTIONS}
        -O2
        -g
        -DGPERF
    )
endif()

# ===================== Dependencies Option =====================
# 开关：OFF=使用brew系统库；ON=FetchContent源码编译（备用）
option(FETCH_DEPS "Build dependencies from source via FetchContent instead of system brew" OFF)

if (NOT FETCH_DEPS)
    # Mode A: Use brew installed packages (CI 默认方案)
    find_package(nlohmann_json REQUIRED)
    find_package(spdlog REQUIRED)
    find_package(fmt REQUIRED)
else()
    # Mode B: FetchContent fallback（复用你fstd工程的宏，此处先声明占位，可直接粘贴find_shared_or_fetch宏进来）
    include(FetchContent)
    message(STATUS "Fetch dependencies from source enabled")
    # Release CI mode: Fetch all thirdparty source, static link
    # fmt static
    set(FMT_INSTALL OFF CACHE BOOL "" FORCE)
    FetchContent_Declare(
        fmt
        GIT_REPOSITORY https://github.com/fmtlib/fmt.git
        GIT_TAG 12.1.0
    )

    # spdlog static, use external fmt
    set(SPDLOG_BUILD_SHARED OFF CACHE BOOL "" FORCE)
    set(SPDLOG_FMT_EXTERNAL ON CACHE BOOL "" FORCE)
    FetchContent_Declare(
        spdlog
        GIT_REPOSITORY https://github.com/gabime/spdlog.git
        GIT_TAG v1.17.0
    )

    # nlohmann_json header-only
    FetchContent_Declare(
        nlohmann_json
        GIT_REPOSITORY https://github.com/nlohmann/json.git
        GIT_TAG v3.12.0
    )

    FetchContent_MakeAvailable(fmt spdlog nlohmann_json)
endif()

# ===================== macOS Frameworks & Link =====================
set(MACOS_FRAMEWORKS
    "-framework ApplicationServices"
    "-framework CoreFoundation"
    "-framework Cocoa"
)

target_link_libraries(fstdict_cgevent_server${BUILD_TYPE} PRIVATE
    ${MACOS_FRAMEWORKS}
    pthread
    spdlog::spdlog
    fmt::fmt
    nlohmann_json::nlohmann_json
)

# Coverage link flags
if (CMAKE_BUILD_TYPE_LOWER STREQUAL "coverage")
    target_link_libraries(fstdict_cgevent_server${BUILD_TYPE} PRIVATE
        -fprofile-arcs
        -ftest-coverage
    )

    add_custom_target(coverage
        COMMAND lcov --base-directory ${CMAKE_SOURCE_DIR} --directory ${CMAKE_BINARY_DIR} --capture --output-file CodeCoverage.info --ignore-errors graph --rc lcov_branch_coverage=1
        COMMAND lcov --remove CodeCoverage.info '/Library/*' '/usr/*' '*/test/*' --output-file CodeCoverage.info.cleaned --rc lcov_branch_coverage=1
        COMMAND genhtml --branch-coverage CodeCoverage.info.cleaned -o CodeCoverageReport --rc lcov_branch_coverage=1
        DEPENDS fstdict_cgevent_server${BUILD_TYPE}
        WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
        COMMENT "Generating code coverage report"
    )

    add_custom_target(open_coverage
        COMMAND open ${CMAKE_BINARY_DIR}/CodeCoverageReport/index.html
        DEPENDS coverage
        COMMENT "Open coverage report in browser"
    )
endif()

# gperf profiling
if (CMAKE_BUILD_TYPE_LOWER STREQUAL "gperf")
    target_link_libraries(fstdict_cgevent_server${BUILD_TYPE} PRIVATE -lprofiler)
endif()