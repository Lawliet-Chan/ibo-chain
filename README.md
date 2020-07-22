# IBO-chain &middot; [![GitHub license](https://img.shields.io/badge/license-GPL3%2FApache2-blue)](LICENSE) [![GitLab Status](https://gitlab.parity.io/parity/substrate/badges/master/pipeline.svg)](https://gitlab.parity.io/parity/substrate/pipelines) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](docs/CONTRIBUTING.adoc)

### 安装部署
1. 安装rust  
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  
source $HOME/.cargo/env

2. 安装 nightly toolchain 和 其对应版本的wasm 
rustup toolchain install nightly-2020-06-27
rustup target add wasm32-unknown-unknown --toolchain nightly-2020-06-27 
(toolchain版本 写在 ibo-chain/bin/node/cli/rust-toolchain 中)  

3. 下载 ibo-chain  
 git clone git@github.com:IBO-Team/ibo-chain.git  

4. 本地编译  
 cd ibo-chain/bin/node/cli && cargo build --release  （也可直接在根目录下编译，但那样需要编译很多不需要的包） 
 编译完之后的二进制文件: ibo-chain/target/release/ibo-chain
 
5. 启动 ibo-chain 
./target/release/ibo-chain --dev

6. 清空链存储
./target/release/ibo-chain purge-chain --dev -y （除了第一次启动以外，每次启动区块链之前都需要清空一次）
 