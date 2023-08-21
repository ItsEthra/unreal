# Unreal
Tooling for generating SDK for UE 4.25+ games.

# 🚨 **Warning** 🚨
## **Project is under development, some offsets may be wrong especially on linux. At the time bitfields are not working properly.**

## Features
- 🗃️ Rust SDK for your Unreal Engine game.
- 🔧 Flexible SDK generation allows you to easily add support for other languages.
- 🔥 Blazingly fast. `[ INFO ] Dumper finished in 1.79s`
- 🌐 Cross platform. Tested on windows and linux.
- ♻️ Automatic elimination of dependency cycles.

## Usage
- Clone the repository with `git clone https://github.com/ItsEthra/unreal && cd unreal`
- Run the dumper and specify process id, FNamePool and TUObjectArray(`FUObjectArray + 0x10`) offsets.\
 `cargo r --release -- -p <PID> -n <FNamePool> -o <TUObjectArray>`
- Done! Your SDK should be in `usdk` folder.
