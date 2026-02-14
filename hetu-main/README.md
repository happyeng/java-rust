# HeTu: High-Performance Centralized Parallel Data-Plane Verification for Hyper-Scale DCN

This repository contains the open-source implementation of **HeTu**, a high-performance centralized parallel data-plane verification system for hyper-scale data center networks (DCN). Hetu was presented at the **2025 IEEE 33rd International Conference on Network Protocols (ICNP)**. For more details, please refer to the [IEEE Xplore article](https://www.computer.org/csdl/proceedings-article/icnp/2025/11192409/2aMnCR93XC8).

## Environment Requirements

### System Requirements
- **Operating System**: Linux/macOS
- **Memory**: Minimum 8GB RAM (16GB+ recommended for large networks)
- **CPU**: Multi-core processor (parallel processing supported)
- **Storage**: At least 1GB free space

### Software Dependencies
- **Rust**: Version 1.70 or higher
- **Cargo**: Rust package manager (included with Rust)

### Installation
1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Clone this repository:
   ```bash
   git clone <repository-url>
   ```
3. Build and run the project:
   ```bash
   cd hetu
   cargo run --release
   ```

## Dataset Input Requirements

The system expects the following input structure in your data directory:

```
data/
├── routes/                    # Device routing tables
│   ├── device1               # Routing table for device1
│   ├── device2               # Routing table for device2
│   └── ...                   # Additional devices
├── topology.json             # Network topology (JSON format)
├── edge_devices              # List of edge devices (one per line)
└── packet_space.json         # Packet space definition (JSON format)
```

By default, the example entry point (`hetu/src/main.rs`) is configured to load a dataset directory like `../data/fattree/fattree10` (relative to `hetu/`). If you want to run on another dataset, update the dataset directory path there accordingly.

## Citation

If you use this code in your research, please cite:

```bibtex
@inproceedings{hetu2025,
  title     = {HeTu: High-Performance Centralized Parallel Data-Plane Verification for Hyper-Scale DCN},
  author    = {Peng, Peng and Sun, Xun and Shen, Zhengtao and Ding, Feiyang and Chen, Jiawei and You, Lizhao and Jiang, Weirong and Tang, Yongping and Luo, Feng},
  booktitle = {Proceedings of the 2025 IEEE 33rd International Conference on Network Protocols (ICNP)},
  year      = {2025},
  organization = {IEEE}
}
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

