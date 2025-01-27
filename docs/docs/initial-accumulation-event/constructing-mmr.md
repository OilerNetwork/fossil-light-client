---
id: constructing-mmr
title: "Constructing the Merkle Mountain Range (MMR)"
---

The MMR is constructed as a perfect binary tree, where each appended block hash is added as a leaf. Internal nodes represent hashes of their child nodes, computed recursively. Append operations update only the nodes along the path to the root, ensuring efficient updates. The MMR root acts as a cryptographic summary of the entire structure and is used for subsequent proof verification.
