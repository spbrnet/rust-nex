# Rust NEX MonoRepo

Rust-Nex (referred to as RNEX,) is a modern, performance first replacement for Nintendo's NEX.

Nintendo didn't exactly build NEX themselves, they built it upon [Quazal](https://web.archive.org/web/20161030054749/http://quazal.com/index.html)'s (now owned by Ubisoft,) [Rendez-vous](https://web.archive.org/web/20161030054839/http://quazal.com/rendez-vous.htm). It is speculated that Nintendo offered NEX in C++.

## Overview
At the highest level, NEX provides a bunch of [services](https://github.com/kinnay/NintendoClients/wiki/NEX-Protocols). Each service provides one or more methods. To achieve this, NEX uses a simple [RMC protocol](https://github.com/kinnay/NintendoClients/wiki/RMC-Protocol) (remote method call). Whenever a game wants to call a method on a service, NEX builds an RMC request and sends it through the underlying connection.

The underlying protocol varies per game and platform. Originally, the purpose of [PRUDP](https://github.com/kinnay/NintendoClients/wiki/PRUDP-Protocol) was to reliably send UDP packets. Starting with Nintendo Switch, NEX also supports TCP and WebSocket as underlying protocol however.

<table>
  <tr>
    <td><b>3DS</b></td><td>Packets are encoded using <a href="https://github.com/kinnay/NintendoClients/wiki/PRUDP-Protocol#v0-format">PRUDP V0</a> or <a href="https://github.com/kinnay/NintendoClients/wiki/PRUDP-Protocol#v1-format">PRUDP V1</a></td>
  </tr>
  <tr>
    <td><b>Wii U</b></td><td>Packets are normally encoded using <a href="https://github.com/kinnay/NintendoClients/wiki/PRUDP-Protocol#v1-format">PRUDP V1</a>. Only one server still uses <a href="https://github.com/kinnay/NintendoClients/wiki/PRUDP-Protocol#v0-format">PRUDP V0</a>: the friends server.</td>
  </tr>
  <tr>
    <td><b>Switch</b></td><td>Switch games use <a href="https://github.com/kinnay/NintendoClients/wiki/PRUDP-Protocol#lite-format">PRUDP Lite</a> on top of WebSockets.</td>
  </tr>
</table>

###### Excerpt sourced from [Kinnay's Nintendo Clients.](https://github.com/kinnay/NintendoClientsWiki/blob/master/NEX-Overview-(Game-Servers).md) Mystiko does not host it's own NEX documentation, instead contributing to third parties already existing documentation.

## Attribution
Rust-Nex is the only service offered by Mystiko that is AGPL. RNEX is free for all to use!

### Thank you to all contributors and all testers! Your help is appreciated :3