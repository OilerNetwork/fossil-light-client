# Fossil L1 to L2 relayer

## Running
1. For local development run [fossil](https://github.com/OilerNetwork/fossil).
2. Export environment variables. For local development export variables from [anvil.env](https://github.com/OilerNetwork/fossil/blob/main/ethereum/anvil.env):
```bash
export $(grep -v '^#' ../fossil/ethereum/anvil.env | xargs)
```
3. Run relayer
```bash
cargo run --release
```
