#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <fstd/fstdx_searcher.hpp>

namespace py = pybind11;
using namespace std;

class FstdEngine
{
public:
    FstdEngine() = default;
    bool add_dict_from_file(const std::string &dict_name, const std::string &fstdx_path)
    {
        return fstdx_searcher.insert(dict_name, fstdx_path);
    }

    bool build_indexes()
    {
        return fstdx_searcher.build_fst_index();
    }

    std::vector<std::string> look_up(const std::string &keyword, const std::string &dict_name)
    {
        return fstdx_searcher.search(keyword, dict_name);
    }

    std::vector<std::string> predictive_search(const std::string &keyword, const std::vector<std::string> &names, size_t top_k = 0)
    {
        return fstdx_searcher.predictive_search(keyword, names);
    }

private:
    fstd::FstdxSearcher fstdx_searcher;
};

// ====================== Pybind 绑定 ======================
PYBIND11_MODULE(fstd_engine, m)
{
    py::class_<FstdEngine>(m, "FstdEngine")
        .def(py::init<>())
        .def("add_dict_from_file", &FstdEngine::add_dict_from_file)
        .def("build_indexes", &FstdEngine::build_indexes)
        .def("look_up", &FstdEngine::look_up)
        .def("predictive_search", &FstdEngine::predictive_search);
}