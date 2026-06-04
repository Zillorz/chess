# Zillorz - Chess
## A fully customizable chess gui

textures and sounds are pulled from assets/

For pieces, first letter 'w' or 'b' signifies colors

and second letter 'K' 'P' 'Q' 'R' 'B' 'N' signifies piece

in png format, for example 'wK.png' is the white king image file

**textures must be 128x128**

For sounds
1) default.ogg - default move sound
2) capture.ogg - capture move sound
3) check.ogg - check move sound
4) castle.ogg - castle move sound

Square textures
128x128
square_1.png & square_2.png

the engine is any uci compatible engine, called from uci.bat

release build is setup with MAIA 1900, an engine meant to act like a human player with around 1900 elo


### Preview:





<img width="596" height="465" alt="Screenshot 2026-06-03 212623" src="https://github.com/user-attachments/assets/8adf5510-1e76-4d3f-8633-7c6ca92c4b1a" />

https://github.com/user-attachments/assets/b4fa20bd-4202-4fe9-97fa-173e48b658f7
(don't analyze this game)

### Building
On windows, either provide the texture files and engine or download them from the newest release.

On linux/mac os, download the texture files from the newest release and the engine seperatly. Should run fine in two-player mode without the engine.
