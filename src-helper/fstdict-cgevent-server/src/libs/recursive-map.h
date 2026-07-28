#ifndef _DBLISP_RECURSIVE_MAP_H_
#define _DBLISP_RECURSIVE_MAP_H_

#include <iostream>
#include <map>
#include <memory>
#include <string>
#include <vector>

namespace dblisp {
class RecTree;

using recursive_map = RecTree;

class KeyType {
  using pointer = std::shared_ptr<std::string>;
  friend class RecTree;
  friend std::ostream& operator<<(std::ostream& outStream, const KeyType& key);
  friend bool operator==(const KeyType& left, const KeyType& right);
  friend bool operator<(const KeyType& left, const KeyType& right);

 public:
  KeyType() : keyPtr_(createPointer("")) {}

  explicit KeyType(const std::string& key) : keyPtr_(createPointer(key)) {}

  KeyType(const KeyType& x) : keyPtr_(x.keyPtr_) {}

  void swap(KeyType& x) noexcept { std::swap(keyPtr_, x.keyPtr_); }

  KeyType& operator=(KeyType x) {
    swap(x);
    return *this;
  }

  ~KeyType() {}

  operator std::string() const { return *keyPtr_; }

  std::string toString() const { return *keyPtr_; }

  void clear() { freePointer(keyPtr_); }

  bool isNull() const { return keyPtr_.get() == nullptr; }

 private:
  pointer createPointer(const std::string& key) {
    return std::make_shared<std::string>(key);
  }

  void freePointer(pointer ptr) { ptr.reset(); }

  const std::string& constRefer() const { return *keyPtr_; }

 private:
  pointer keyPtr_;
};

inline std::ostream& operator<<(std::ostream& outStream, const KeyType& key) {
  outStream << key.constRefer();
  return outStream;
}

inline bool operator==(const KeyType& left, const KeyType& right) {
  return left.constRefer() == right.constRefer();
}

inline bool operator!=(const KeyType& left, const KeyType& right) {
  return (!(left == right));
}

inline bool operator<(const KeyType& left, const KeyType& right) {
  return left.constRefer() < right.constRefer();
}

inline bool operator>=(const KeyType& left, const KeyType& right) {
  return (!(left < right));
}

inline bool operator>(const KeyType& left, const KeyType& right) {
  return (right < left);
}

inline bool operator<=(const KeyType& left, const KeyType& right) {
  return (!(left > right));
}

class ValType {
  friend class RecTree;

  friend std::ostream& operator<<(std::ostream& outStream,
                                  const ValType& value);

 public:
  explicit ValType(const std::string& valStr) : valStr_(valStr) {}

  ValType(ValType&& x) : valStr_(std::move(x.valStr_)) {}

  ValType(const ValType& x) : valStr_(x.valStr_) {}

  ValType& operator=(ValType x) {
    swap(x);
    return *this;
  }

  void swap(ValType& x) noexcept { valStr_.swap(x.valStr_); }

  operator std::string() const { return asString(); }

  bool asBool() const { return valStr_ != "false"; }

  int asInt() const { return std::stoi(valStr_); }

  long int asLInt() const { return std::stol(valStr_); }

  long long int asLLInt() const { return std::stoll(valStr_); }

  unsigned int asUInt() const { return std::stoul(valStr_); }

  unsigned long long int asULLInt() const { return std::stoull(valStr_); }

  std::string asString() const { return valStr_; }

  char asChar() const { return valStr_.front(); }

  float asFloat() const { return std::stof(valStr_); }

  double asDouble() const { return std::stod(valStr_); }

  long double asLDouble() const { return std::stold(valStr_); }

 private:
  std::string valStr_;
};

inline std::ostream& operator<<(std::ostream& outStream, const ValType& value) {
  outStream << value.valStr_;
  return outStream;
}

struct RecTree_const_iterator {
  friend class RecTree;

 public:
  typedef std::bidirectional_iterator_tag iterator_category;
  typedef RecTree value_type;
  typedef const value_type& reference;
  typedef const value_type* pointer;
  typedef ptrdiff_t difference_type;

  typedef typename std::map<KeyType, RecTree*>::iterator node_type;
  typedef RecTree_const_iterator self;

  RecTree_const_iterator() = default;

  RecTree_const_iterator(const node_type& node) : node_(node) {}

  RecTree_const_iterator(const self& x) : node_(x.node_) {}

  bool operator==(const self& x) const { return node_ == x.node_; }

  bool operator!=(const self& x) const { return (!(operator==(x))); }

  self& operator=(const self& x) {
    node_ = x.node_;
    return (*this);
  }

  reference operator*() const { return *node_->second; }

  pointer operator->() const { return (&(operator*())); }

  self& operator--() {
    --node_;
    return *this;
  }

  self& operator++() {
    ++node_;
    return *this;
  }

  self operator++(int) {
    auto temp = *this;
    operator++();
    return temp;
  }

  self operator--(int) {
    auto temp = *this;
    operator--();
    return (temp);
  }

 protected:
  node_type node_;
};

struct RecTree_iterator : public RecTree_const_iterator {
  typedef RecTree_const_iterator base_iterator;

  typedef std::bidirectional_iterator_tag iterator_category;
  typedef typename base_iterator::value_type value_type;
  typedef value_type& reference;
  typedef value_type* pointer;
  typedef ptrdiff_t difference_type;

  typedef typename base_iterator::node_type node_type;
  typedef RecTree_iterator self;

  RecTree_iterator() = default;

  RecTree_iterator(const node_type& node) : base_iterator(node) {}

  RecTree_iterator(const self& x) : base_iterator(x.node_) {}

  self& operator=(const self& x) {
    this->node_ = x.node_;
    return *this;
  }

  reference operator*() const { return (*this->node_->second); }

  pointer operator->() const { return (&(operator*())); }

  self& operator--() {
    --this->node_;
    return *this;
  }

  self& operator++() {
    ++this->node_;
    return *this;
  }

  self operator--(int) {
    auto temp = *this;
    operator--();
    return temp;
  }

  self operator++(int) {
    auto temp = *this;
    operator++();
    return temp;
  }
};

class DbLispParser;

class RecTree {
  friend class DbLispParser;

 public:
  using key_type = KeyType;
  using link_type = RecTree*;
  enum VALUE_TYPE { VALUE, VALUE_VECTOR, RECTREE, INITAL };
  union value_type {
    ValType* value_;
    std::vector<ValType>* valueVec_;
    std::map<key_type, link_type>* children_;
  };

 public:
  typedef RecTree_iterator iterator;
  typedef RecTree_const_iterator const_iterator;

  typedef std::map<key_type, link_type>::iterator map_iterator;
  typedef std::map<key_type, link_type>::const_iterator map_const_iterator;

 public:
  RecTree() : key_(), valueStatus_(INITAL) { nodeValue_.children_ = nullptr; }

  ~RecTree() { clear(); }

  explicit RecTree(const std::string& key) : key_(key), valueStatus_(INITAL) {
    nodeValue_.children_ = nullptr;
  }

  RecTree(RecTree&& x)
      : key_(std::move(x.key_)),
        nodeValue_(x.nodeValue_),
        valueStatus_(x.valueStatus_) {
    x.valueStatus_ = INITAL;
    x.nodeValue_.children_ = nullptr;
  }

  RecTree(const RecTree& x) { copy(x); }

  RecTree& operator=(RecTree x) {
    swap(x);
    return *this;
  }

  void swap(RecTree& x) noexcept {
    key_.swap(x.key_);
    std::swap(nodeValue_, x.nodeValue_);
    std::swap(valueStatus_, x.valueStatus_);
  }

  std::ostream& formatLisp(std::ostream& outStream) const {
    outStream << formatLisp();
    return outStream;
  }

  std::string formatLisp() const {
    std::string lispStr;
    formatLisp(this, 0, lispStr);
    return lispStr;
  }

  const std::vector<ValType>& valueVector() const {
    if (isSingleValue()) const_cast<link_type>(this)->moveValToVec();
    return refValVector();
  }

  std::vector<ValType>& valueVector() {
    if (isSingleValue()) moveValToVec();
    return refValVector();
  }

  key_type key() const { return key_; }

  iterator begin() { return refChildren().begin(); }

  const_iterator begin() const { return refChildren().begin(); }

  iterator end() { return refChildren().end(); }

  const_iterator end() const { return refChildren().end(); }

  const_iterator cbegin() const { return refChildren().begin(); }

  const_iterator cend() const { return refChildren().end(); }

  iterator erase(const_iterator pos) {
    freeTree(pos.node_->second);
    return refChildren().erase(pos.node_);
  }

  size_t erase(const std::string& key) {
    iterator pos = find(key);
    if (pos == end()) return 0;
    erase(pos);
    return 1;
  }

  iterator erase(const_iterator first, const_iterator last) {
    for (auto pos = first; pos != last; ++pos) {
      freeTree(pos.node_->second);
    }
    return refChildren().erase(first.node_, last.node_);
  }

  const_iterator find(const std::string& key) const {
    return refChildren().find(key_type(key));
  }

  iterator find(const std::string& key) {
    return refChildren().find(key_type(key));
  }

  bool empty() const { return size() == 0; }

  const RecTree& at(const std::string& key) const {
    return *refChildren().at(key_type(key));
  }

  RecTree& at(const std::string& key) {
    return *refChildren().at(key_type(key));
  }

  size_t count() const { return count(this); }

  size_t size() const { return isTree() ? refChildren().size() : 0; }

  template <typename RecType>
  std::pair<iterator, bool> insert(RecType&& recTree) {
    return emplace(std::forward<RecType>(recTree));
  }

  std::pair<iterator, bool> insert(const RecTree& recTree) {
    return emplace(recTree);
  }

  template <typename Iter>
  void insert(Iter first, Iter last) {
    for (; first != last; ++first) insert(*first);
  }

  void insert(std::initializer_list<RecTree> il) {
    insert(il.begin(), il.end());
  }

  template <typename... types>
  std::pair<iterator, bool> emplace(const std::string& key, types&&... args) {
    std::pair<map_iterator, bool> prIB;
    switch (valueStatus_) {
      case VALUE:
        freeValue();
        break;
      case VALUE_VECTOR:
        freeValVector();
        break;
      case RECTREE:
        prIB = refChildren().emplace(key, nullptr);
        if (prIB.second) {
          prIB.first->second =
              createTree(prIB.first->first, std::forward<types>(args)...);
        }
        return {prIB.first, prIB.second};
        break;
      default:;
    }
    valueStatus_ = RECTREE;
    nodeValue_.children_ = createChildren();
    link_type linkTree = createTree(key, std::forward<types>(args)...);
    prIB = refChildren().emplace(linkTree->key_, linkTree);
    return {prIB.first, prIB.second};
  }

  template <typename RecType>
  std::pair<map_iterator, bool> emplace(RecType&& recTree) {
    std::pair<map_iterator, bool> prIB;
    switch (valueStatus_) {
      case VALUE:
        freeValue();
        break;
      case VALUE_VECTOR:
        freeValVector();
        break;
      case RECTREE:
        prIB = refChildren().emplace(recTree.key_, nullptr);
        if (prIB.second) {
          prIB.first->second = createTree(std::forward<RecType>(recTree));
        }
        return {prIB.first, prIB.second};
        break;
      default:;
    }
    valueStatus_ = RECTREE;
    nodeValue_.children_ = createChildren();
    prIB = refChildren().emplace(recTree.key_,
                                 createTree(std::forward<RecType>(recTree)));
    return {prIB.first, prIB.second};
  }

  RecTree& operator[](const std::string& key) { return *(emplace(key).first); }

  void clear() {
    switch (valueStatus_) {
      case VALUE:
        this->freeValue();
        break;
      case VALUE_VECTOR:
        this->freeValVector();
        break;
      case RECTREE:
        this->clearChildren();
        break;
      default:;
    }
    valueStatus_ = INITAL;
  }

 public:
  ValType& value(const size_t index = 0) const {
    if (isSingleValue()) {
      if (index == 0) {
        return refValue();
      }
    }
    return (*nodeValue_.valueVec_)[index];
  }

  bool isValue() const {
    return isSingleValue() || valueStatus_ == VALUE_VECTOR;
  }

  bool isMap() const { return isTree(); }

  void pushValue(const std::string& val) {
    switch (valueStatus_) {
      case VALUE:
        moveValToVec();
      case VALUE_VECTOR:
        refValVector().emplace_back(val);
        return;
        break;
      default:;
    }
    clearNodeValue();
    nodeValue_.value_ = createValue(val);
    valueStatus_ = VALUE;
  }

 public:
  template <typename Iter>
  void assign(Iter begin, Iter end) {
    clearNodeValue();
    nodeValue_.valueVec_ = createValVector(begin, end);
    valueStatus_ = VALUE_VECTOR;
  }

  const ValType& operator[](const size_t index) const { return value(index); }

  ValType& operator[](const size_t index) { return value(index); }

 private:
  size_t count(const RecTree* const tree) const {
    size_t ret = 0;
    switch (tree->valueStatus_) {
      case INITAL:
      case VALUE:
      case VALUE_VECTOR:
        ret = 1;
        break;
      case RECTREE:
        ret = tree->countChilren() + 1;
        break;
      default:;
    }
    return ret;
  }

  size_t countChilren() const {
    size_t ret = 0;
    for (const auto& p : this->refChildren()) {
      ret += count(p.second);
    }
    return ret;
  }

 private:
  bool formatLisp(const RecTree* const tPtr, size_t preSpaceCount,
                  std::string& lispStr) const {
    std::string lispVal;
    bool newline = false;
    size_t spaceCount = preSpaceCount;
    switch (tPtr->valueStatus_) {
      case INITAL:
        lispStr.append("(")
            .append(toLispVal(tPtr->refRealKey()))
            .push_back(')');
        break;
      case VALUE:
        lispStr.append("(")
            .append(toLispVal(tPtr->refRealKey()))
            .append(" ")
            .append(toLispVal(tPtr->refRealVal()))
            .push_back(')');
        break;
      case VALUE_VECTOR:
        lispStr.append("(").append(toLispVal(tPtr->refRealKey()));
        for (const auto& val : tPtr->refValVector()) {
          lispStr.append(" ").append(toLispVal(val));
        }
        lispStr.push_back(')');
        break;
      case RECTREE:
        lispVal = toLispVal(tPtr->refRealKey());
        lispStr.append("(").append(lispVal).push_back(' ');
        if (tPtr->empty()) {
          lispStr.push_back(')');
        } else if (tPtr->size() == 1) {
          preSpaceCount += lispVal.size() + 2;
          newline = (formatLisp(tPtr->begin().node_->second, preSpaceCount,
                                lispStr) ||
                     newline);
          if (newline) {
            lispStr.push_back('\n');
            lispStr.append(std::string(spaceCount, ' '));
          }
          lispStr.push_back(')');
        } else {
          newline = true;
          preSpaceCount += lispVal.size() + 2;
          formatLisp(tPtr->begin().node_->second, preSpaceCount, lispStr);
          for (auto iter = ++tPtr->begin(); iter != tPtr->end(); ++iter) {
            lispStr.push_back('\n');
            lispStr.append(std::string(preSpaceCount, ' '));
            formatLisp(iter.node_->second, preSpaceCount, lispStr);
          }
          lispStr.push_back('\n');
          lispStr.append(std::string(spaceCount, ' ')).push_back(')');
        }
        break;
      default:;
    }
    return newline;
  }

  std::string toLispVal(const std::string& originVal) const {
    std::string val("\"");
    val.reserve(originVal.size());
    for (const auto& c : originVal) {
      if (c == '"') {
        val.append("\\\"");
      } else {
        val.push_back(c);
      }
    }
    val.push_back('"');
    return val;
  }

  bool isTree() const { return valueStatus_ == RECTREE; }

  void moveValToVec() {
    ValType tempVal = std::move(*nodeValue_.value_);
    freeValue();
    nodeValue_.valueVec_ = createValVector();
    nodeValue_.valueVec_->emplace_back(std::move(tempVal));
    valueStatus_ = VALUE_VECTOR;
  }

  bool isSingleValue() const { return valueStatus_ == VALUE; }

  link_type copy(const RecTree& x) {
#ifdef _DBLISP_TEST_DEBUG_
    std::cout << "copy: " << x.key_ << std::endl;
#endif
    this->key_ = key_type(x.refRealKey());
    this->valueStatus_ = x.valueStatus_;
    switch (x.valueStatus_) {
      case VALUE:
        this->nodeValue_.value_ = this->createValue(x.refRealVal());
        break;
      case VALUE_VECTOR:
        this->nodeValue_.valueVec_ = this->createValVector(x.refValVector());
        break;
      case RECTREE:
        this->nodeValue_.children_ = this->copyChildren(x.refChildren());
        break;
      default:;
    }
    return this;
  }

  void clearChildren() {
    for (const auto& p : this->refChildren()) {
      freeTree(p.second);
    }
    this->refChildren().clear();
    this->freeChildren();
  }

  void clearNodeValue() {
    switch (valueStatus_) {
      case VALUE:
        freeValue();
        break;
      case VALUE_VECTOR:
        freeValVector();
        break;
      case RECTREE:
        clearChildren();
        break;
      default:;
    }
    valueStatus_ = INITAL;
  }

  ValType* createValue(const std::string& val) {
#ifdef _DBLISP_TEST_DEBUG_
    std::cout << "createValue: " << val << std::endl;
#endif
    return new ValType(val);
  }

  std::string& refRealKey() const { return *key_.keyPtr_; }

  void freeValue() {
#ifdef _DBLISP_TEST_DEBUG_
    std::cout << "freeValue: " << value() << std::endl;
#endif
    delete nodeValue_.value_;
  }

  template <typename... types>
  std::vector<ValType>* createValVector(types&&... args) {
    return new std::vector<ValType>(std::forward<types>(args)...);
  }

  void freeValVector() { delete nodeValue_.valueVec_; }

  ValType& refValue() const { return *nodeValue_.value_; }

  std::string& refRealVal() const { return refValue().valStr_; }

  std::vector<ValType>& refValVector() const { return *nodeValue_.valueVec_; }

  std::map<key_type, link_type>& refChildren() const {
    return *nodeValue_.children_;
  }

  std::map<key_type, link_type>* copyChildren(
      const std::map<key_type, link_type>& chidlren) {
    auto child = createChildren();
    for (const auto& p : chidlren) {
      link_type tree = createTree(*p.first.keyPtr_);
      child->emplace(tree->key_, tree->copy(*p.second));
    }
    return child;
  }

  template <typename... types>
  std::map<key_type, link_type>* createChildren(types&&... args) {
    return new std::map<key_type, link_type>(std::forward<types>(args)...);
  }

  void freeChildren() { delete nodeValue_.children_; }

  template <typename... types>
  link_type createTree(types... args) {
    link_type tree = new RecTree(std::forward<types>(args)...);
#ifdef _DBLISP_TEST_DEBUG_
    std::cout << "createTree: " << tree->key_ << std::endl;
#endif
    return tree;
  }

  void freeTree(link_type treePtr) {
#ifdef _DBLISP_TEST_DEBUG_
    std::cout << "freeTree: " << treePtr->key_ << std::endl;
#endif
    treePtr->clear();
    delete treePtr;
  }

 private:
  key_type key_;
  union value_type nodeValue_;
  VALUE_TYPE valueStatus_;
};
}  // namespace dblisp
#undef DBLISP_TEST_DEBUG
#endif