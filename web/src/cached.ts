/**
 * `f`, remembering each result by its argument: for lookups whose answer
 * never changes, such as a field's core spec or a Display's binding.
 */
export function cached<K, V>(f: (key: K) => V): (key: K) => V {
  const results = new Map<K, V>();
  return (key) => {
    if (results.has(key)) return results.get(key) as V;
    const value = f(key);
    results.set(key, value);
    return value;
  };
}
