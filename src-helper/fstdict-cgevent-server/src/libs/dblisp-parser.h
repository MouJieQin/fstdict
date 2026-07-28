#ifndef _DBLISP_DBLISP_PARSER_H_
#define _DBLISP_DBLISP_PARSER_H_
#include <fstream>
#include <stack>

#include "recursive-map.h"

namespace dblisp {

enum WordType { LEFT_PARENTHESIS, RIGHT_PARENTHESIS, STRING_VALUE, VARIABLE };

class DbLispParser;

class DbLispWord {
  friend class DbLispParser;
  friend std::ostream& operator<<(std::ostream& outStream,
                                  const DbLispWord& word);

 public:
  DbLispWord(std::string&& value, WordType wordType)
      : value_(std::move(value)), wordType_(wordType) {}

 private:
  std::string value_;
  WordType wordType_;
};

// std::ostream& operator<<(std::ostream& outStream, const DbLispWord& word) {
//   outStream << word.value_;
//   return outStream;
// }

class DbLispParser {
  enum map_type { MAP_INIT, MAP_MAP, MAP_VALUE };
  using link_type = recursive_map::link_type;

 public:
  ~DbLispParser() { clearMapStk(); }

  bool lispToRecMap(const std::string& lispFile, recursive_map& rmap) {
    lispFile_ = lispFile;
    std::vector<std::string> lispFileVec;
    if (!copyToFile(lispFile, lispFileVec)) {
      return false;
    }
    recursive_map rmapTemp(rmap.key());
    clearMapStk();
    mapStk.push(std::make_pair(&rmapTemp, MAP_MAP));
    if (!lispToRecMap(lispFileVec, rmapTemp)) {
      clearMapStk();
      return false;
    }
    rmap.swap(rmapTemp);
    clearMapStk();
    return true;
  }

 private:
  void clearMapStk() {
    if (mapStk.empty()) {
      return;
    }
    for (; mapStk.size() != 1;) {
      mapStk.top().first->clear();
      mapStk.top().first->freeTree(mapStk.top().first);
      mapStk.pop();
    }
    mapStk.pop();
  }

  bool lispToRecMap(std::vector<std::string>& lispFileVec,
                    recursive_map& rmap) {
    std::vector<DbLispWord> wordVec;
    if (!lispWords(lispFileVec, wordVec)) {
      return false;
    }
#ifdef _DBLISP_TEST_DEBUG_
    std::cout << "--------------start------------------------\n";
    for (const auto& word : wordVec) {
      std::cout << word << " ";
    }
    std::cout << "\n----------------end----------------------" << std::endl;
#endif
    return wordToRecMap(wordVec, rmap);
  }

  bool wordToRecMap(std::vector<DbLispWord>& wordVec, recursive_map& rmap) {
    recursive_map::iterator iter;
    std::pair<link_type, map_type> top;
    std::pair<recursive_map::iterator, bool> prIB;
    map_type mapType;
    for (size_t index = 0; index != wordVec.size();) {
      switch (wordVec[index].wordType_) {
        case LEFT_PARENTHESIS:
          index += 1;
          if (index == wordVec.size()) {
            return errorLog("`(` not close");
          }
          switch (wordVec[index].wordType_) {
            case LEFT_PARENTHESIS:
              return errorLog("`(` must have a key");
              break;
            case RIGHT_PARENTHESIS:
              return errorLog("`()` is invalid syntax");
              break;
            case STRING_VALUE:
              mapStk.push(std::make_pair(
                  rmap.createTree(std::move(wordVec[index].value_)), MAP_INIT));
              index += 1;
              break;
            case VARIABLE:
              if (rmap.empty() ||
                  (iter = rmap.find(wordVec[index].value_)) == rmap.end()) {
                return errorLog("Variable `(" + wordVec[index].value_ +
                                ")` is Undefined");
              }
              switch (iter->valueStatus_) {
                case recursive_map::VALUE_TYPE::VALUE:
                case recursive_map::VALUE_TYPE::VALUE_VECTOR:
                  mapType = map_type::MAP_VALUE;
                  break;
                case recursive_map::VALUE_TYPE::RECTREE:
                  mapType = map_type::MAP_MAP;
                  break;
                case recursive_map::VALUE_TYPE::INITAL:
                  mapType = map_type::MAP_INIT;
                  break;
                default:;
              }
              mapStk.push(std::make_pair(rmap.createTree(*iter), mapType));
              index += 1;
              break;
            default:;
          }
          break;
        case RIGHT_PARENTHESIS:
          if (mapStk.size() == 1) {
            return errorLog("`) not close");
          }
          top = mapStk.top();
          mapStk.pop();
          if (mapStk.top().second != MAP_MAP &&
              mapStk.top().second != MAP_INIT) {
            top.first->freeTree(top.first);
            return errorLog("The definition of `" +
                            mapStk.top().first->refRealKey() +
                            "` is ambiguous");
          }
          prIB = mapStk.top().first->emplace(std::move(*top.first));
          top.first->freeTree(top.first);
          if (!prIB.second) {
            return errorLog("duplicate key `" + prIB.first->refRealKey() + "`");
          }
          mapStk.top().second = MAP_MAP;
          index += 1;
          break;
        case STRING_VALUE:
          if (mapStk.size() == 1) {
            return errorLog("`\"" + wordVec[index].value_ +
                            "\" is invalid syntax");
          }
          if (mapStk.top().second != MAP_VALUE &&
              mapStk.top().second != MAP_INIT) {
            return errorLog("The definition of `" +
                            mapStk.top().first->refRealKey() +
                            "` is ambiguous");
          }
          mapStk.top().first->pushValue(wordVec[index].value_);
          mapStk.top().second = MAP_VALUE;
          index += 1;
          break;
        case VARIABLE:
          if (mapStk.top().second != MAP_VALUE &&
              mapStk.top().second != MAP_INIT) {
            return errorLog("The definition of `" +
                            mapStk.top().first->refRealKey() +
                            "` is ambiguous");
          }
          if (rmap.empty() ||
              (iter = rmap.find(wordVec[index].value_)) == rmap.end()) {
            return errorLog("Variable `" + wordVec[index].value_ +
                            "` is Undefined");
          }
          if (iter->isValue()) {
            if (iter->isSingleValue()) {
              mapStk.top().first->pushValue(iter->refRealVal());
            } else {
              for (const auto& val : iter->refValVector()) {
                mapStk.top().first->pushValue(val);
              }
            }
          } else if (iter->isTree()) {
            return errorLog("Variable `" + wordVec[index].value_ +
                            "` can not converted into values");
          } else {
          }
          index += 1;
          break;
        default:;
      }
    }
    return mapStk.size() == 1 ? true : errorLog("`(` not close");
  }

  bool lispWords(const std::vector<std::string>& lispFileVec,
                 std::vector<DbLispWord>& wordVec) {
    size_t lineIndex = 0, index = 0;
    size_t openLineIndex = 0, openIndex = 0, closeIndex = 0;
    bool quotClose = true;
    std::string strValue;
    std::vector<DbLispWord> wordVecTemp;
    for (;;) {
      if (lineIndex >= lispFileVec.size()) {
        if (!quotClose) {
          return errorIndexLog(openLineIndex, openIndex, "`\" not close");
        }
        wordVec.swap(wordVecTemp);
        return true;
      }
      if (index >= lispFileVec[lineIndex].size()) {
        index = 0;
        lineIndex += 1;
        if (!quotClose) {
          strValue.push_back('\n');
        }
      } else {
        if (!quotClose) {
          if (lispFileVec[lineIndex][index] == '"') {
            quotClose = true;
            wordVecTemp.emplace_back(std::move(strValue), STRING_VALUE);
            strValue.clear();
            index += 1;
          } else {
            strValue.push_back(lispFileVec[lineIndex][index]);
            closeIndex = index + 1;
            for (; closeIndex != lispFileVec[lineIndex].size(); ++closeIndex) {
              strValue.push_back(lispFileVec[lineIndex][closeIndex]);
              if (lispFileVec[lineIndex][closeIndex] == '"') {
                if (lispFileVec[lineIndex][closeIndex - 1] == '\\') {
                  strValue.pop_back();
                  strValue.back() = '"';
                } else {
                  quotClose = true;
                  strValue.pop_back();
                  wordVecTemp.emplace_back(std::move(strValue), STRING_VALUE);
                  strValue.clear();
                  break;
                }
              }
            }
            index = closeIndex + 1;
          }
        } else {
          const char c = lispFileVec[lineIndex][index];
          switch (c) {
            case '(':
              wordVecTemp.emplace_back("(", LEFT_PARENTHESIS);
              index += 1;
              break;
            case ')':
              wordVecTemp.emplace_back(")", RIGHT_PARENTHESIS);
              index += 1;
              break;
            case ';':
              lineIndex += 1;
              index = 0;
              break;
            case '"':
              quotClose = false;
              openLineIndex = lineIndex;
              openIndex = index;
              closeIndex = index + 1;
              for (; closeIndex != lispFileVec[lineIndex].size();
                   ++closeIndex) {
                strValue.push_back(lispFileVec[lineIndex][closeIndex]);
                if (lispFileVec[lineIndex][closeIndex] == '"') {
                  if (lispFileVec[lineIndex][closeIndex - 1] == '\\') {
                    strValue.pop_back();
                    strValue.back() = '"';
                  } else {
                    quotClose = true;
                    strValue.pop_back();
                    wordVecTemp.emplace_back(std::move(strValue), STRING_VALUE);
                    strValue.clear();
                    break;
                  }
                }
              }
              index = closeIndex + 1;
              break;
            default:
              if (!isspace(c)) {
                for (size_t startIndex = index;; ++index) {
                  if (index == lispFileVec[lineIndex].size()) {
                    wordVecTemp.emplace_back(
                        lispFileVec[lineIndex].substr(startIndex), VARIABLE);
                    break;
                  }
                  if (lispFileVec[lineIndex][index] == ')') {
                    wordVecTemp.emplace_back(
                        lispFileVec[lineIndex].substr(startIndex,
                                                      index - startIndex),
                        VARIABLE);
                    wordVecTemp.emplace_back(")", RIGHT_PARENTHESIS);
                    break;
                  } else if (isspace(lispFileVec[lineIndex][index])) {
                    wordVecTemp.emplace_back(
                        lispFileVec[lineIndex].substr(startIndex,
                                                      index - startIndex),
                        VARIABLE);
                    break;
                  }
                }
              }
              index += 1;
              break;
          }
        }
      }
    }
    return true;
  }

 private:
  bool copyToFile(const std::string& inputFile,
                  std::vector<std::string>& lispFileVec) {
    std::ifstream inf(inputFile);
    if (!inf.is_open()) return openErrorLog(inputFile);
    std::string buf;
    buf.reserve(100);
    std::vector<std::string> lispFileVecImple;
    for (; getline(inf, buf);) {
      lispFileVecImple.emplace_back(std::move(buf));
    }
    inf.close();
    lispFileVec.swap(lispFileVecImple);
    return true;
  }

 private:
  std::ostream& errorLog(std::ostream& ostream) const {
    ostream << "dblisp: parser: error: " << lispFile_ << ":";
    return ostream;
  }

  bool errorLog(const std::string& logInfo) const {
    errorLog(std::cerr) << logInfo << std::endl;
    return false;
  }

  bool errorIndexLog(const size_t lineIndex, const size_t index,
                     const std::string& logInfo) {
    errorLog(std::cerr) << lineIndex + 1 << ":" << index + 1 << ':' << logInfo
                        << std::endl;
    return false;
  }

  bool openErrorLog(const std::string& fileName) const {
    errorLog(std::cerr) << "open error: " << fileName << std::endl;
    return false;
  }

 private:
  std::string lispFile_;
  std::stack<std::pair<link_type, map_type>> mapStk;
 };

}  // namespace dblisp

#endif