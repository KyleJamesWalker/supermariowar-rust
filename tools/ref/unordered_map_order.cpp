// Reference for server/unordered_map.rs: libc++ std::unordered_map iteration order and bucket_count
// after each operation of a fixed pseudo-random script. Output goes to unordered_map_order.txt.
// Build: clang++ -std=c++17 -O2 unordered_map_order.cpp -o /tmp/umo && /tmp/umo > unordered_map_order.txt
#include <cstdint>
#include <cstdio>
#include <unordered_map>

static uint32_t lcg_state = 12345;
static uint32_t lcg() {
    lcg_state = lcg_state * 1103515245u + 12345u;
    return (lcg_state >> 8) & 0xFFFFFF;
}

template <class K>
static void dump(const char* tag, int step, const std::unordered_map<K, int>& m) {
    printf("%s %d bc=%zu n=%zu:", tag, step, m.bucket_count(), m.size());
    for (const auto& kv : m)
        printf(" %llu=%d", (unsigned long long)kv.first, kv.second);
    printf("\n");
}

template <class K>
static void run(const char* tag, uint32_t keyRange, int steps, bool sequential) {
    std::unordered_map<K, int> m;
    K nextKey = 1;
    for (int i = 0; i < steps; i++) {
        uint32_t r = lcg();
        uint32_t op = r % 20;
        if (op < 12) {
            K key = sequential ? nextKey++ : (K)(lcg() % keyRange);
            m[key] = i;
        } else if (op < 19) {
            K key = sequential ? (K)(1 + lcg() % (uint32_t)nextKey) : (K)(lcg() % keyRange);
            m.erase(key);
        } else {
            m.clear();
        }
        dump(tag, i, m);
    }
}

int main() {
    run<uint32_t>("u32rand", 200, 900, false);
    run<uint32_t>("u32seq", 0, 900, true);
    run<uint64_t>("u64rand", 1000000, 600, false);
    return 0;
}
