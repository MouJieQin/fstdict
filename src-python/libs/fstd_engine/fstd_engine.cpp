#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <fstd/fstdx_searcher.h>
#include <fstd/fstdd_reader.h>
#include <memory>

namespace py = pybind11;
using namespace std;

class FstdEngine
{
public:
    FstdEngine() = default;
    FstdEngine(const std::string &meta_json_path) : fstdx_searcher(meta_json_path, 0)
    {
        if (!fstdx_searcher)
        {
            throw std::runtime_error("Failed to load FstdEngine from meta JSON: " + meta_json_path);
        }
    }
    ~FstdEngine() = default;

    bool extract(const string &name, const string &file_path,
                 const std::string &dst_dir = "data")
    {
        return fstdx_searcher.extract(name, file_path, dst_dir);
    }

    bool default_extract(const string &name, const string &file_path)
    {
        return fstdx_searcher.extract(name, file_path);
    }

    bool save_to_disk(const std::string &meta_json_path)
    {
        return fstdx_searcher.save_to_disk(meta_json_path);
    }

    void add_dict_from_file(const std::string &dict_name, const std::string &fstdx_path)
    {
        fstdx_searcher.insert_if_not_exists(dict_name, fstdx_path);
    }

    std::vector<std::string> look_up(const std::string &keyword, const std::string &dict_name)
    {
        return fstdx_searcher.search(keyword, dict_name);
    }

    std::vector<std::string> common_prefix_search(const std::string &keyword, const std::vector<std::string> &names)
    {
        return fstdx_searcher.common_prefix_search(keyword, names);
    }

    std::string longest_common_prefix_search(const std::string &keyword, const std::vector<std::string> &names)
    {
        size_t prefix_len = fstdx_searcher.longest_common_prefix_search(keyword, names);
        return keyword.substr(0, prefix_len);
    }

    std::vector<std::string> predictive_search(const std::string &keyword, const std::vector<std::string> &names, size_t top_k = 0)
    {
        return fstdx_searcher.predictive_search(keyword, names);
    }

    std::vector<std::string> edit_distance_search(const std::string &keyword, const std::vector<std::string> &names, size_t edit_distance = 1)
    {
        return fstdx_searcher.edit_distance_search(keyword, names, edit_distance);
    }

    std::vector<std::string>
    prefix_distance_search(const std::string &keyword,
                           const std::vector<std::string> &names,
                           size_t max_distance) const
    {
        return fstdx_searcher.prefix_distance_search(keyword, names, max_distance);
    }

    std::vector<std::string> suggest(const std::string &keyword, const std::vector<std::string> &names)
    {
        return fstdx_searcher.suggest(keyword, names);
    }

    std::pair<std::vector<std::string>, std::string>
    regex_search(const std::string &pattern,
                 const std::vector<std::string> &names) const
    {
        return fstdx_searcher.regex_search(pattern, names);
    }

private:
    fstd::FstdxSearcher fstdx_searcher;
};

// ====================== Pybind 绑定 ======================
PYBIND11_MODULE(fstd_engine, m)
{
    py::class_<FstdEngine>(m, "FstdEngine")
        .def(py::init<>())
        .def(py::init<const std::string &>())
        .def("save_to_disk", &FstdEngine::save_to_disk)
        .def("extract", &FstdEngine::extract)
        .def("default_extract", &FstdEngine::default_extract)
        .def("add_dict_from_file", &FstdEngine::add_dict_from_file)
        .def("look_up", &FstdEngine::look_up)
        .def("common_prefix_search", &FstdEngine::common_prefix_search)
        .def("longest_common_prefix_search", &FstdEngine::longest_common_prefix_search)
        .def("predictive_search", &FstdEngine::predictive_search)
        .def("edit_distance_search", &FstdEngine::edit_distance_search)
        .def("prefix_distance_search", &FstdEngine::prefix_distance_search)
        .def("suggest", &FstdEngine::suggest)
        .def("regex_search", &FstdEngine::regex_search);
}