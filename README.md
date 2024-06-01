First edition.

It works, but... there's still a long road ahead!

Approx. CCRL rating: 1750

### Progress status:

Pre-calculations - Complete!

Board - Complete! (~24m nps on perft)

Engine - Complete! (~300k to 800k nps; Inspired by: [BBC engine](https://github.com/maksimKorzh))

Eval - Complete! (Inspired by: Kaissa engine, also used PeSTO eval)

Comms - Partially complete (Musthave features work, xboard protocol only)

### TODO:

Full xboard (CECP v2) and/or UCI support

Resolve issues (incompatibilities, enhancements, there are a lot)

Opening books, endgame tables support

### Game samples:

https://lichess.org/eAJeuTrS/white/ (40/300 vs BBC 1.2 as White)

https://lichess.org/SvtcAEVm/black/ (40/30 vs Really Smart Human(tm) with 2k FIDE and ~inf time as Black)

https://lichess.org/GfIy7Uxr/ (40/5 vs self, hyperbullet basically)

https://lichess.org/V4Zq4DPA/white/ (40/900* vs Roce 0.0390 as White)

https://lichess.org/kblZvQqi/black/ (40/60* vs Cinnamon 1.2c as Black)

https://lichess.org/rZ6fKTkP/black/ (40/180 vs Fox 1.1 as Black)

#####* - used much better PC for hosting this game

### "I want to play against it!"

1. Download and install xboard (WinBoard) or any interface with CECP v2 protocol supported

2. Add Akira CE latest .exe from github releases (e.g. http://hgm.nubati.net/xboard/winboard/help/html/20.htm)

3. Options -> Common Engine Settings, set Hash Size as 512 MB (it's hardcoded for now, sorry)

Akira doesn't use any opening books or endspiel tables.

Set time controls to whatever odds, you may also want to disable pondering in Options -> General.

I'll deploy it as a lichess bot sooner or later.

Enjoy!

### Special thanks (to All The Fellas)

[MegaMorpeh](https://lichess.org/@/MegaMorpeh)

[Konstantin_Russia](https://lichess.org/@/Konstantin_Russia)

[Kivicatte](https://lichess.org/@/Kivicatte)

[Ravile](https://lichess.org/@/Ravile)

[paradoxine](https://lichess.org/@/paradoxine)

mirkuriit

[Avernial](https://ru.stackoverflow.com/users/178429/avernial)

##### ...and to All The other Fellas

### "Is it really 'First' edition though?"

Nope. https://github.com/N1ckn1ght/CCE

But we won't count this one.