#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <vector>
#include <string>
#include <fstream>
#include <unordered_map>
#include <unordered_set>
#include <algorithm>
#include <cstring>
#include <cctype>

namespace py = pybind11;
using namespace std;

// 快速小写转换
string to_lower(const string &s)
{
    string res;
    res.reserve(s.size());
    for (unsigned char c : s)
        res += tolower(c);
    return res;
}

// 极简 Levenshtein 距离
int levenshtein(const char *a, const char *b)
{
    int m = 0, n = 0;
    const char *pa = a, *pb = b;
    while (*pa++)
        m++;
    while (*pb++)
        n++;
    vector<int> dp(n + 1);
    for (int i = 0; i <= n; i++)
        dp[i] = i;
    for (int i = 1; i <= m; i++)
    {
        int prev = dp[0];
        dp[0] = i;
        for (int j = 1; j <= n; j++)
        {
            int temp = dp[j];
            if (a[i - 1] == b[j - 1])
                dp[j] = prev;
            else
                dp[j] = 1 + min({prev, dp[j], dp[j - 1]});
            prev = temp;
        }
    }
    return dp[n];
}

// 单词实体：存储真实字符串 + 归属词典
struct WordEntry
{
    string word;
    // vector<const string *> dict_names; // 归属词典列表（指针，避免复制）
    WordEntry(string w) : word(std::move(w)) {}
};

// ====================== 核心类 ======================
class WordStorage
{
private:
    // 全局唯一单词池：单词字符串 -> 单词实体
    unordered_map<string, unique_ptr<WordEntry>> _word_pool;

    // 词典：词典名 -> 单词列表（指针）
    unordered_map<string, vector<const WordEntry *>> _dict_words;

    // 视图缓存：词典组合key -> 去重排序后的单词视图
    unordered_map<string, vector<const WordEntry *>> _view_cache;

    unordered_map<string, unique_ptr<string>> _dict_names;

private:
    // 禁止拷贝/移动
    WordStorage(const WordStorage &) = delete;
    WordStorage &operator=(const WordStorage &) = delete;
    WordStorage(WordStorage &&) = delete;
    WordStorage &operator=(WordStorage &&) = delete;

    // 生成视图唯一key
    string get_view_key(const vector<string> &dicts) const
    {
        vector<string> sorted = dicts;
        sort(sorted.begin(), sorted.end());
        string key;
        for (const auto &d : sorted)
            key += d + "|";
        return key;
    }

    // 获取/创建去重排序的视图
    vector<const WordEntry *> get_view(const vector<string> &dicts)
    {
        string key = get_view_key(dicts);
        auto it = _view_cache.find(key);
        if (it != _view_cache.end())
            return it->second;

        unordered_set<const WordEntry *> temp_set;
        for (const auto &dict : dicts)
        {
            auto dict_it = _dict_words.find(dict);
            if (dict_it == _dict_words.end())
                continue;
            for (const auto *entry : dict_it->second)
                temp_set.insert(entry);
        }

        vector<const WordEntry *> view(temp_set.begin(), temp_set.end());
        sort(view.begin(), view.end(), [](const WordEntry *a, const WordEntry *b)
             { return a->word < b->word; });

        _view_cache[key] = std::move(view);
        return _view_cache[key];
    }

public:
    WordStorage() = default;
    ~WordStorage() = default;

    // 添加词典：自动去重单词、记录归属词典
    size_t add_dict(const string &dict_name, const vector<string> &words)
    {
        if (_dict_names.count(dict_name))
            return 0; // 已存在同名词典，忽略
        _dict_names[dict_name] = make_unique<string>(dict_name);
        auto &word_list = _dict_words[dict_name];
        word_list.reserve(words.size());

        for (const string &word : words)
        {
            // 单词不存在则创建
            if (!_word_pool.count(word))
            {
                _word_pool[word] = make_unique<WordEntry>(word);
            }
            WordEntry *entry = _word_pool[word].get();
            // 记录归属
            // entry->dict_names.push_back(_dict_names[dict_name].get());
            word_list.push_back(entry);
        }

        // 添加新词典后清空视图缓存（避免脏数据）
        _view_cache.clear();
        return words.size();
    }

    size_t add_dict_from_file(const string &dict_name, const string &file_path)
    {
        vector<string> words;
        ifstream infile(file_path);
        if (!infile.is_open())
            return 0;
        string line;
        while (getline(infile, line))
        {
            if (!line.empty())
                words.push_back(line);
        }
        infile.close();
        return add_dict(dict_name, words);
    }

    // 前缀搜索
    vector<string> prefix_search(const string &keyword, vector<string> use_dicts, size_t limit)
    {
        vector<string> res;
        auto view = get_view(use_dicts);
        size_t kw_len = keyword.size();

        for (const auto *entry : view)
        {
            if (res.size() >= limit)
                break;
            const string &w = entry->word;
            if (w.size() >= kw_len && w.compare(0, kw_len, keyword) == 0)
            {
                res.push_back(w);
            }
        }
        return res;
    }

    // 包含搜索
    vector<string> contains_search(const string &keyword, vector<string> use_dicts, size_t limit)
    {
        vector<string> res;
        auto view = get_view(use_dicts);

        for (const auto *entry : view)
        {
            if (res.size() >= limit)
                break;
            if (entry->word.find(keyword) != string::npos)
            {
                res.push_back(entry->word);
            }
        }
        return res;
    }

    // 模糊搜索（已修复，可启用）
    vector<string> fuzzy_search(const string &keyword, vector<string> use_dicts, size_t limit)
    {
        string key = to_lower(keyword);
        vector<pair<int, string>> candidates;
        auto view = get_view(use_dicts);

        for (const auto *entry : view)
        {
            int d = levenshtein(key.c_str(), to_lower(entry->word).c_str());
            candidates.emplace_back(d, entry->word);
        }

        // 按距离升序排序
        sort(candidates.begin(), candidates.end());

        vector<string> res;
        for (size_t i = 0; i < min(limit, candidates.size()); i++)
        {
            res.push_back(candidates[i].second);
        }
        return res;
    }

    // 模糊包含搜索（已修复，可启用）
    vector<string> fuzzy_contains_search(const string &keyword, vector<string> use_dicts, size_t limit)
    {
        string key = to_lower(keyword);
        vector<pair<int, string>> candidates;
        auto view = get_view(use_dicts);

        for (const auto *entry : view)
        {
            string lower_w = to_lower(entry->word);
            if (lower_w.find(key) == string::npos)
                continue;
            int d = levenshtein(key.c_str(), lower_w.c_str());
            candidates.emplace_back(d, entry->word);
        }

        sort(candidates.begin(), candidates.end());

        vector<string> res;
        for (size_t i = 0; i < min(limit, candidates.size()); i++)
        {
            res.push_back(candidates[i].second);
        }
        return res;
    }
};

// ====================== Pybind 绑定 ======================
PYBIND11_MODULE(word_engine, m)
{
    py::class_<WordStorage>(m, "WordStorage")
        .def(py::init<>())
        .def("add_dict", &WordStorage::add_dict)
        .def("add_dict_from_file", &WordStorage::add_dict_from_file)
        .def("prefix_search", &WordStorage::prefix_search)
        .def("contains_search", &WordStorage::contains_search)
        .def("fuzzy_search", &WordStorage::fuzzy_search)
        .def("fuzzy_contains_search", &WordStorage::fuzzy_contains_search);
}