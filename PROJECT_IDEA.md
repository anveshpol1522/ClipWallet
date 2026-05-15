## Inspiration
I have been shipping products to YC companies; attending weekly hackathons; working on cutting edge research on AGI; working 6-12-7 to build something new/ solve a new problem but still I don't know how to use an excel (spreadsheet). I would rather use a .csv file with a python code. I know the pain of working with Ctrl + C a million times.

## What it does
Maintains a large set of data copies through keys on the keyboard. That's it!

## How we built it
We looked to build a dynamic clipboard on Windows and MacOS without actually depending on it. Utility can be directly downloaded from the terminal or zip file. It can support various sorts of data types. It should always run on the system 24/7 (persistent background service). We use pointer variables if possible orelse we store it temporarily. These locations are either chosen static (the number you allocate) or dynamically (the latest use cases in 123...). Providing addressable memory slots (1-9). Capacity to switch between memory, disk storage and encrypted storage (if needed). Best to develop application over Rust language. 

**How will we deal with the memory?**
We use RAM (to keep it quick) and occasionally switch to Local File Storage (for persistence).
Why RAM? Extremely fast memory access, zero disk I/O, simple implementation, no serialisation needed, and perfect for temporary data.
Why not RAM? Lost data on RAM refresh/ crashing or system sleep/ shut down/ restart (no persistence), memory limit, and not reliable for long term usage.
Solution to RAM model? Use local file storage periodically. We get a place to add large files (only if pointers do not work), store when RAM clears out, available cross platform etc. Pushes back to RAM when new new memory is allocated.

**Features planned to implement (Static):**
1. Cmd + Opt + Tab: Navigation, assigns to Cmd + C and can be accesed using Cmd + V
2. Cmd + Opt + Shift + Tab: Reverse navigation, assigns to Cmd + C and can be accesed using Cmd + V
3. Cmd + Opt + X + 1-9: Cut and store objects
4. Cmd + Opt + C + 1-9: Copy and store objects
5. Cmd + Opt + V + 1-9: Paste stored objects
6. Cmd + Opt + Tab/ Shift-Tab + Esc: Delete data stored

**Features planned to implement (Dynamic):**
1. Cmd + Opt + Tab: Navigation, assigns to Cmd + C and can be accesed using Cmd + V
2. Cmd + Opt + Shift + Tab: Reverse navigation, assigns to Cmd + C and can be accesed using Cmd + V
3. Cmd + Opt + X: Cut and store objects, placed in a dynamic array.
4. Cmd + Opt + C: Copy and store objects, placed in a dynamic array.
5. Cmd + Opt + Tab/ Shift-Tab + Esc: Delete data stored
6. Cmd + Opt + V: Alternative to using Cmd + V
Here the data is stored dynamically with the latest one used at the start (as Cmd + V).

## Challenges we ran into
Over-writing/ nullifying keys on MacOS.
Figuring out special key purposes. 
Implementing complex data structures.
Understanding crux of OS.
Furnishing application into deployable product. 

## Accomplishments that we're proud of
Cracking implementation despite strong conflicting MacOS architecture.

## What we learned
There are many things in this world that start from a vision to build. It need not be to compete with Claude, Gemini, or ChatGPT but just solve a small problem that can bring value to the people around you.

## What's next for ClipWallet
Build new keyboard native shortcuts for special applications.
