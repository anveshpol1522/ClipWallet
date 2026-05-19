# Manual Installation Guide

Welcome! If the quick-install script didn't work for you, or if you prefer to build the application from scratch, this guide will walk you through the manual installation process step-by-step. 

**You do not need to be a developer to follow these steps.** Just open your computer's terminal (Command Prompt on Windows, Terminal on macOS/Linux) and follow along.

---

## Step 1: Install Prerequisites

Before downloading ClipWallet, your computer needs two tools:
1. **Git:** To download the code.
2. **Rust:** The programming language used to build ClipWallet.

### For Linux
Open your **Terminal** and run the following commands:

1. **Install Git:**
   ```sh
   sudo apt update && sudo apt install git -y

2. Install Rust:
    Bash

    curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh


3. **Apply Changes:** Restart your terminal or run this command so your system recognizes Rust:
   ```sh
   source $HOME/.cargo/env

### For Windows

    Install Git: Download and run the installer from git-scm.com. Keep all default settings.

    Install Rust: Download and run rustup-init.exe from rustup.rs. Follow the on-screen prompts (pressing 1 and Enter for the default installation).

### For macOS

Open your Terminal and paste the following commands one by one, pressing Enter after each:

    Install Git:
    Bash

    git --version

    (If Git isn't installed, a prompt will appear asking you to install the Xcode Command Line Tools. Agree and let it install).

    Install Rust:
    Bash

    curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh


Step 2: Download ClipWallet

Now that you have the required tools, you can download the code.

    Open your terminal.

    Copy and paste the following command to download the repository:
    Bash

    git clone [https://github.com/shaaravraghu/ClipWallet.git](https://github.com/shaaravraghu/ClipWallet.git)

    Move into the newly downloaded folder:
    Bash

    cd ClipWallet



## Step 3: Build and Run the Application

With the code downloaded, you can now build the application. 

1. While inside the `ClipWallet` folder in your terminal, run:
   ```sh
   cargo build --release
   

(This process might take a few minutes as it downloads and compiles the necessary components. Don't worry if you see a lot of text scrolling by!)

    Once the build is complete, you can find the ready-to-use application inside the target/release/ folder.

    To run the application immediately from the terminal, use:
    Bash

    cargo run --release


**Congratulations! You have successfully installed and run ClipWallet manually.**
