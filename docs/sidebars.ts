import type { SidebarsConfig } from "@docusaurus/plugin-content-docs";

const sidebars: SidebarsConfig = {
  tutorialSidebar: [
    {
      type: "doc",
      id: "introduction", // FIX: Use correct filename as ID
      label: "Introduction",
    },
    {
      type: "category",
      label: "System Overview",
      collapsed: false,
      items: [
        "system-overwiew/high-level-architecture",
        "system-overwiew/components-and-data-flow",
        "system-overwiew/key-technologies",
      ],
    },
    {
      type: "category",
      label: "Initial Accumulation Event",
      collapsed: false,
      items: [
        "initial-accumulation-event/intro",
        "initial-accumulation-event/data-source",
        "initial-accumulation-event/database-setup",
        "initial-accumulation-event/block-headers-validation",
        "initial-accumulation-event/batch-selection",
        "initial-accumulation-event/constructing-mmr",
        "initial-accumulation-event/generating-proofs",
        "initial-accumulation-event/onchain-submission",
      ],
    },
    {
      type: "category",
      label: "The Light Client",
      collapsed: false,
      items: [
        "updating-light-client/intro",
        "updating-light-client/relaying-block-hashes",
        "updating-light-client/initiating-update",
      ],
    },
  ],
};

export default sidebars;
