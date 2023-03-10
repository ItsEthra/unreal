# Unreal
Tooling for generating SDK for UE 4.25+ games.

# 🚨 **Warning** 🚨
## **Project is under development, some offsets may be wrong especially on linux. At the time bitfields are not working properly.**

## Features
- 🗃️ Rust SDK for your Unreal Engine game.
- 🔧 Flexible SDK generation allows you to easily add support for other languages.
- 🔥 Blazingly fast. `[INFO  dumper] Finished in 1.319184584s`
- 🌐 Cross platform. Tested on windows and linux.

## Usage
- Clone the repository with `git clone https://github.com/ItsEthra/unreal && cd unreal`
- Run the dumper and specify process id, FNamePool(GNames) and FUObjectArray(GObjects) addresses.\
 `cargo r -- <PID> -N <GNames> -O <GObjects>`
- Done! Your SDK should be in `usdk` folder.
