# Unreal
Tooling for generating SDK for UE 4.25+ games.

## Features
- 🗃️ Rust SDK for your Unreal Engine game.
- 🔧 Flexible SDK generation allows you to easily add support for other languages.
- 🔥 Blazingly fast. Dumped 1400 packages and removed dependency cycles in 15 seconds.
- 🌐 Cross platform. Tested on windows and linux.
- ♻️ Automatic elimination of dependency cycles.
- 👾 Method generation to assist in calling in-game functions.
- 🕸️ Dependency graph generation that can be saved to a file. Sample files can be found in [here](/samples).

## Usage
- Clone the repository with `git clone https://github.com/ItsEthra/unreal && cd unreal`
- Run the dumper and specify process id, FNamePool and TUObjectArray(`FUObjectArray + 0x10`) offsets.\
 `cargo r --release -- -p <PID> -N <FNamePool> -O <TUObjectArray>`
- Done! Your SDK should be in `usdk` folder.
