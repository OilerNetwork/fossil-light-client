# Constructing the Merkle Mountain Range

The MMR is constructed as a perfect binary tree, where each appended block hash is added as a leaf. It can be seen either as a list of perfectly balanced binary trees or a single binary tree that would have been truncated from the top right.

A Merkle Mountain Range (MMR) is strictly append-only: elements are added from the left to the right, adding a parent as soon as 2 children exist, filling up the range accordingly.

Internal nodes represent hashes of their child nodes, computed recursively. Append operations update only the nodes along the path to the root, ensuring efficient updates.

The MMR root acts as a cryptographic summary of the entire structure and is used for subsequent proof verification.

Unlike a traditional Merkle tree, an MMR lacks a single inherent root, so a process called "bagging the peaks" is used to compute one.
After identifying all peaks, bagging them means iteratively hashing them from right to left with the MMR’s total size as a prefix. For an MMR of size N with peaks p1, p2, and p3, the final root hash is computed as:

```text
P = keccak256(N || keccak256(N || Node(p3) || Node(p2)) || Node(p1))
```

Here, "||" denotes concatenation, and keccak256 is used for hashing.
