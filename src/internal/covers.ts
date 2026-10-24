/**
 * Compile-time checks for hand-kept value lists of generated unions:
 * `type _Check = Assert<Covers<Union, typeof LIST>>;` fails to compile when
 * the list misses a member of the union.
 */
export type Covers<Union, List extends readonly Union[]> = [
  Exclude<Union, List[number]>,
] extends [never]
  ? true
  : false;

export type Assert<T extends true> = T;
