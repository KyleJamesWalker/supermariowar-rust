//! libc++ `std::unordered_map` with unique keys: same bucket layout, prime rehash policy and
//! `std::hash`, so iteration order matches the C++ server (`SMWServer::rooms`).
//! Checked against `tools/ref/unordered_map_order.cpp`.

const NIL: usize = usize::MAX;
/// `__first_node_` (before-begin) as a bucket predecessor.
const BEFORE_BEGIN: usize = usize::MAX - 1;

/// libc++ `std::hash` for integral keys: the value itself as `size_t`.
pub trait LibcxxHash {
    fn libcxx_hash(&self) -> usize;
}

macro_rules! identity_hash {
    ($($t:ty),*) => {$(
        impl LibcxxHash for $t {
            fn libcxx_hash(&self) -> usize {
                *self as usize
            }
        }
    )*};
}
identity_hash!(u8, u16, u32, u64, usize, i8, i16, i32, i64);

struct Node<K, V> {
    next: usize,
    hash: usize,
    key: K,
    value: Box<V>,
}

pub struct UnorderedMap<K, V> {
    nodes: Vec<Option<Node<K, V>>>,
    free: Vec<usize>,
    first: usize,
    buckets: Vec<usize>,
    size: usize,
}

fn is_hash_power2(bc: usize) -> bool {
    bc > 2 && (bc & (bc - 1)) == 0
}

fn constrain_hash(h: usize, bc: usize) -> usize {
    if bc & bc.wrapping_sub(1) == 0 {
        h & bc.wrapping_sub(1)
    } else if h < bc {
        h
    } else {
        h % bc
    }
}

fn next_hash_pow2(n: usize) -> usize {
    if n < 2 {
        n
    } else {
        1usize << (usize::BITS - (n - 1).leading_zeros())
    }
}

/// `std::__next_prime`: the smallest prime >= n (0 stays 0).
fn next_prime(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut p = n.max(2);
    loop {
        let mut prime = true;
        let mut d = 2;
        while d * d <= p {
            if p % d == 0 {
                prime = false;
                break;
            }
            d += 1;
        }
        if prime {
            return p;
        }
        p += 1;
    }
}

impl<K: LibcxxHash + Eq, V> Default for UnorderedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: LibcxxHash + Eq, V> UnorderedMap<K, V> {
    pub fn new() -> Self {
        UnorderedMap { nodes: Vec::new(), free: Vec::new(), first: NIL, buckets: Vec::new(), size: 0 }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    fn node(&self, i: usize) -> &Node<K, V> {
        self.nodes[i].as_ref().unwrap()
    }

    fn node_mut(&mut self, i: usize) -> &mut Node<K, V> {
        self.nodes[i].as_mut().unwrap()
    }

    fn next_of(&self, p: usize) -> usize {
        if p == BEFORE_BEGIN {
            self.first
        } else {
            self.node(p).next
        }
    }

    fn set_next(&mut self, p: usize, n: usize) {
        if p == BEFORE_BEGIN {
            self.first = n;
        } else {
            self.node_mut(p).next = n;
        }
    }

    fn find_node(&self, key: &K) -> usize {
        let bc = self.bucket_count();
        if bc == 0 || self.size == 0 {
            return NIL;
        }
        let hash = key.libcxx_hash();
        let chash = constrain_hash(hash, bc);
        let pn = self.buckets[chash];
        if pn == NIL {
            return NIL;
        }
        let mut nd = self.next_of(pn);
        while nd != NIL {
            let n = self.node(nd);
            if !(n.hash == hash || constrain_hash(n.hash, bc) == chash) {
                break;
            }
            if n.hash == hash && n.key == *key {
                return nd;
            }
            nd = n.next;
        }
        NIL
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.find_node(key) != NIL
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let nd = self.find_node(key);
        if nd == NIL {
            None
        } else {
            Some(&self.node(nd).value)
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let nd = self.find_node(key);
        if nd == NIL {
            None
        } else {
            Some(&mut self.node_mut(nd).value)
        }
    }

    /// `operator[]`: the existing value, or `make()` inserted where libc++ would put it.
    pub fn index_or_insert_with(&mut self, key: K, make: impl FnOnce() -> V) -> &mut V {
        let found = self.find_node(&key);
        if found != NIL {
            return &mut self.node_mut(found).value;
        }

        let hash = key.libcxx_hash();
        let node = Node { next: NIL, hash, key, value: Box::new(make()) };
        let h = match self.free.pop() {
            Some(i) => {
                self.nodes[i] = Some(node);
                i
            }
            None => {
                self.nodes.push(Some(node));
                self.nodes.len() - 1
            }
        };

        let mut bc = self.bucket_count();
        if (self.size + 1) as f32 > bc as f32 * 1.0f32 {
            let grow = 2 * bc + !is_hash_power2(bc) as usize;
            let need = ((self.size + 1) as f32 / 1.0f32).ceil() as usize;
            self.rehash(grow.max(need));
            bc = self.bucket_count();
        }
        let chash = constrain_hash(hash, bc);

        let pn = self.buckets[chash];
        if pn == NIL {
            let next = self.first;
            self.node_mut(h).next = next;
            self.first = h;
            self.buckets[chash] = BEFORE_BEGIN;
            if next != NIL {
                let nb = constrain_hash(self.node(next).hash, bc);
                self.buckets[nb] = h;
            }
        } else {
            let next = self.next_of(pn);
            self.node_mut(h).next = next;
            self.set_next(pn, h);
        }
        self.size += 1;
        &mut self.node_mut(h).value
    }

    /// `m[key] = value`
    pub fn insert(&mut self, key: K, value: V) {
        let mut value = Some(value);
        let slot = self.index_or_insert_with(key, || value.take().unwrap());
        if let Some(v) = value {
            *slot = v;
        }
    }

    fn rehash(&mut self, mut n: usize) {
        if n == 1 {
            n = 2;
        } else if n & n.wrapping_sub(1) != 0 {
            n = next_prime(n);
        }
        let bc = self.bucket_count();
        if n > bc {
            self.do_rehash(n);
        } else if n < bc {
            let need = (self.size as f32 / 1.0f32).ceil() as usize;
            n = n.max(if is_hash_power2(bc) { next_hash_pow2(need) } else { next_prime(need) });
            if n < bc {
                self.do_rehash(n);
            }
        }
    }

    fn do_rehash(&mut self, bucket_count: usize) {
        self.buckets = vec![NIL; bucket_count];
        if bucket_count == 0 {
            return;
        }

        let mut pp = BEFORE_BEGIN;
        let mut cp = self.first;
        if cp == NIL {
            return;
        }

        let mut chash = constrain_hash(self.node(cp).hash, bucket_count);
        self.buckets[chash] = pp;
        let mut phash = chash;
        pp = cp;
        cp = self.node(cp).next;
        while cp != NIL {
            chash = constrain_hash(self.node(cp).hash, bucket_count);
            if chash == phash {
                pp = cp;
            } else if self.buckets[chash] == NIL {
                self.buckets[chash] = pp;
                pp = cp;
                phash = chash;
            } else {
                let np = cp;
                let after = self.node(np).next;
                self.set_next(pp, after);
                let b = self.buckets[chash];
                let bnext = self.next_of(b);
                self.node_mut(np).next = bnext;
                self.set_next(b, cp);
            }
            cp = self.next_of(pp);
        }
    }

    /// `erase(key)`
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let cn = self.find_node(key);
        if cn == NIL {
            return None;
        }

        let bc = self.bucket_count();
        let chash = constrain_hash(self.node(cn).hash, bc);
        let mut pn = self.buckets[chash];
        while self.next_of(pn) != cn {
            pn = self.next_of(pn);
        }

        let cnext = self.node(cn).next;
        if pn == BEFORE_BEGIN || constrain_hash(self.node(pn).hash, bc) != chash {
            if cnext == NIL || constrain_hash(self.node(cnext).hash, bc) != chash {
                self.buckets[chash] = NIL;
            }
        }
        if cnext != NIL {
            let nhash = constrain_hash(self.node(cnext).hash, bc);
            if nhash != chash {
                self.buckets[nhash] = pn;
            }
        }
        self.set_next(pn, cnext);
        self.size -= 1;

        let node = self.nodes[cn].take().unwrap();
        self.free.push(cn);
        Some(*node.value)
    }

    /// `clear()`: keeps the bucket count.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.free.clear();
        self.first = NIL;
        for b in self.buckets.iter_mut() {
            *b = NIL;
        }
        self.size = 0;
    }

    /// Iteration in libc++ order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        let mut cur = self.first;
        std::iter::from_fn(move || {
            if cur == NIL {
                return None;
            }
            let n = self.node(cur);
            cur = n.next;
            Some((&n.key, &*n.value))
        })
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u32);
    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
            (self.0 >> 8) & 0xFFFFFF
        }
    }

    fn dump<K: LibcxxHash + Eq + Copy + Into<u64>>(out: &mut String, tag: &str, step: i32, m: &UnorderedMap<K, i32>) {
        out.push_str(&format!("{} {} bc={} n={}:", tag, step, m.bucket_count(), m.len()));
        for (k, v) in m.iter() {
            out.push_str(&format!(" {}={}", (*k).into(), v));
        }
        out.push('\n');
    }

    fn run<K: LibcxxHash + Eq + Copy + Into<u64> + TryFrom<u32>>(rng: &mut Lcg, out: &mut String, tag: &str, keyRange: u32, steps: i32, sequential: bool)
    where
        <K as TryFrom<u32>>::Error: std::fmt::Debug,
    {
        let mut m: UnorderedMap<K, i32> = UnorderedMap::new();
        let mut nextKey: u32 = 1;
        for i in 0..steps {
            let r = rng.next();
            let op = r % 20;
            if op < 12 {
                let key = if sequential {
                    nextKey += 1;
                    nextKey - 1
                } else {
                    rng.next() % keyRange
                };
                m.insert(K::try_from(key).unwrap(), i);
            } else if op < 19 {
                let key = if sequential { 1 + rng.next() % nextKey } else { rng.next() % keyRange };
                m.remove(&K::try_from(key).unwrap());
            } else {
                m.clear();
            }
            dump(out, tag, i, &m);
        }
    }

    #[test]
    fn matches_libcxx_iteration_order() {
        let expected = include_str!("../../tools/ref/unordered_map_order.txt");
        let mut rng = Lcg(12345);
        let mut out = String::new();
        run::<u32>(&mut rng, &mut out, "u32rand", 200, 900, false);
        run::<u32>(&mut rng, &mut out, "u32seq", 0, 900, true);
        run::<u64>(&mut rng, &mut out, "u64rand", 1000000, 600, false);
        for (n, (a, b)) in expected.lines().zip(out.lines()).enumerate() {
            assert_eq!(a, b, "line {}", n + 1);
        }
        assert_eq!(expected.lines().count(), out.lines().count());
    }
}
