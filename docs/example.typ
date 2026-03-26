// Struktura maturitního projektu: Komunikační aplikace (p2p-chat)
// Toto je pouze kostra dokumentu – samotný obsah patří do main.typ

= Titulní strana
- Název školy a logo
- Název práce: "Komunikační aplikace zaměřující se na soukromí pomocí decentralizace"
- Autor, třída, obor
- Vedoucí práce
- Datum

= Prohlášení
- Samostatné vypracování
- Podpis, místo, datum

= Poděkování
- Vedoucí práce
- Případní další pomocníci

= Abstrakt
- Stručný popis cíle práce
- Použité technologie (Rust, libp2p, SQLite, ratatui)
- Výsledky a přínos práce
- Klíčová slova: p2p, chat, decentralizace, Rust, libp2p, soukromí

= Obsah (automaticky generovaný)

= 1. Úvod
- Cíl práce: decentralizovaná chatovací aplikace bez centrálního serveru
- Motivace: ochrana soukromí, metadata, centralizované služby
- Struktura aplikace – přehled čtyř komponent:
  - `p2pchat-core` – backendový daemon
  - `p2pchat-relay` – relay server pro NAT traversal
  - `p2pchat-http` – HTTP tracker pro registraci jmen
  - `p2pchat-tui` – terminálové uživatelské rozhraní
- Postup vývoje (analytická → návrhová → implementace → testování)

= 2. Pozadí a analýza existujících řešení
== 2.1 Existující chatovací systémy
- Matrix – federated, open-source, serverová architektura
- Signal – centralizovaný, end-to-end šifrování
- Keet (HolePunch) – P2P, closed-source, bez persistentního úložiště zpráv
- Srovnávací tabulka: open-source / P2P / persistentní zprávy / NAT traversal

== 2.2 Problém a motivace
- Ochrana soukromí: šifrování přenosu zpráv
- Uchovávání metadat u centralizovaných služeb
- Cíl: přímá komunikace mezi peery (direct / relayed)

= 3. Použité technologie
== 3.1 Rust
- Systémový jazyk, bezpečnost paměti bez GC
- Ownership a borrowing systém
- Proč Rust: výkon, bezpečnost, ekosystém (Cargo, crates.io)

== 3.2 libp2p (rust-libp2p)
- Modulární P2P síťová knihovna (původ: Protocol Labs / IPFS)
- Přehled použitých behavior modulů:
  - mDNS – discovery v lokální síti
  - Request-Response – protokol pro zprávy a přátelství
  - Identify – identifikace peerů
  - Relay Client – NAT traversal přes relay server
  - DCUTR – přímé spojení přes hole-punching
  - TCP a QUIC – transportní protokoly
- Swarm jako centrální koordinátor chování

== 3.3 Tokio (asynchronní runtime)
- Asynchronní I/O pro Rust
- Event loop, task scheduling
- Použití v projektu: síťové operace, Unix socket server, souběžné tasky

== 3.4 Ratatui (terminálové UI)
- Knihovna pro TUI v Rustu (fork tui-rs)
- Widgety, layouty, stylizace
- Integrace s Tokio (crossterm events)
- Použití: kontakty panel, chat okno, friends tab, status bar, setup wizard

== 3.5 SQLite (tokio-rusqlite)
- Serverless relační databáze, soubor na disku
- Schéma aplikace: contacts, messages, friends, pending_friend_requests, names, channels
- Migrace schématu při startu

== 3.6 Nix
- Reprodukovatelné buildování a správa závislostí
- flake.nix pro vývojové prostředí
- `nix build` / `nix run` pro jednoduché spuštění

= 4. Návrh aplikace
== 4.1 Architektura – přehled komponent
- Diagram: TUI ↔ Unix Socket ↔ p2pchat-core ↔ libp2p swarm ↔ peers
- Workspace struktura (Cargo workspace se třemi crates)
  - `p2pchat-types` – sdílené typy
  - `p2pchat-core` – backend daemon
  - `tui` – frontend

== 4.2 IPC komunikace (Unix Socket)
- Protokol: length-prefixed postcard serializace
- Typy zpráv z TUI do core: `UiClientRequest` / `UiClientEvent`
- Typy zpráv z core do TUI: `WriteEvent`
- Přehled klíčových eventů:
  - SendMessage, LoadChatlogPage
  - SendFriendRequest, AcceptFriendRequest, DenyFriendRequest
  - SearchUsername, LoadFriends, LoadIncomingFriendRequests
  - DiscoverMdnsContact, PeerDisconnected, RelayServerConnection

== 4.3 Databázové schéma
- Tabulky a vztahy (ER diagram nebo popis):
  - `contacts` – PeerId, vazba na names a channel
  - `names` – jméno s TTL (24 h)
  - `channels` – privátní kanály 1:1
  - `messages` – UUID, obsah, channel_id, created_at
  - `friends` – potvrzená přátelství
  - `pending_friend_requests` – čekající žádosti (incoming / outgoing)
- Migrace schématu při startu

== 4.4 Konfigurace a nastavení
- Uložení: `~/.config/p2pchat/settings` (JSON)
- Databáze: `~/.local/share/p2pchat/db`
- Povinné hodnoty: Name (username), KeyPair (Ed25519)
- Setup wizard (`setup` binary) – první spuštění, registrace jména

== 4.5 Identita a kryptografie
- Ed25519 klíčový pár jako identita peera
- PeerId odvozeno z veřejného klíče pomocí SHA256
- Podpisování zpráv pro ověření autenticity (`signable.rs`)
- HTTP tracker: registrace jména s podepsaným požadavkem

= 5. Implementace
== 5.1 p2pchat-core (backend daemon)
- Vstupní bod `main.rs`: Unix socket server, dispatcher eventů
- CLI argumenty: `-r` relay adresa, `-t` tracker adresa
- Graceful shutdown

=== 5.1.1 Síťová vrstva (`network.rs`)
- Konfigurace libp2p Swarm (behavior stack)
- EventLoop: zpracování swarm eventů a příkazů z TUI
- Client: async API pro posílání příkazů do EventLoop
- Buffering požadavků při nedostupnosti peera

=== 5.1.2 Protokoly
- `/direct-message/1` – Request-Response protokol pro zprávy
  - `DirectMessageRequest` (obsah zprávy)
  - `MessageResponse` (Ack / NotFriends)
- `/friends/1` – protokol pro správu přátelství
  - `FriendRequest::AddFriend` – žádost o přátelství
  - `FriendRequest::AcceptFriend { decision }` – přijetí/odmítnutí
  - Přenos jména peera při mDNS discovery

=== 5.1.3 Databázová vrstva (`db/`)
- CRUD operace pro contacts, messages, friends
- Stránkování historie zpráv

== 5.2 tui/ (terminálové UI)
- Dva binární cíle: `tui` (hlavní UI) a `setup` (wizard)
- Stavy aplikace: Normal (navigace), Editing (psaní zprávy)
- Záložky: Contacts (chat) a Friends (správa přátel)

=== 5.2.1 Setup wizard (`setup.rs`)
- Generování keypair při prvním spuštění
- Zadání uživatelského jména
- Kontrola dostupnosti jména přes HTTP tracker
- Registrace jména podpisem přes HTTP tracker
- Uložení nastavení

=== 5.2.2 Hlavní UI (`main.rs`, `ui.rs`)
- Panel kontaktů (levý sidebar): mDNS peers, přátelé
- Chat oblast: zobrazení zpráv, textový vstup
- Friends záložka: vyhledávání, čekající žádosti, příchozí žádosti
- Status bar: stav relay připojení, aktuální peer

== 5.3 HTTP Tracker klient (`tracker.rs`)
- Registrace jména (POST s podpisem)
- Vyhledání jména (GET peer_id podle jména)
- Kontrola dostupnosti jména

== 5.4 Klíčové datové toky
=== 5.4.1 Odeslání zprávy
- Uživatel napíše zprávu → TUI → Unix socket → core → libp2p dial → DirectMessageRequest → SQLite → Ack → TUI

=== 5.4.2 Žádost o přátelství
- Vyhledání uživatele (tracker) → SendFriendRequest → libp2p → FriendRequest → příjemce rozhodne → AcceptFriend → SQLite obou stran → notifikace TUI

=== 5.4.3 Peer discovery (mDNS)
- mDNS event → nový kontakt do SQLite → vyžádání jména přes `/friends/1.0.0` → TUI notifikace

=== 5.4.4 Připojení přes relay a DCUTR
- Připojení na relay server → circuit relay adresa → pokus o DCUTR hole-punch → přímé spojení (UDP/QUIC nebo TCP)

= 6. Bezpečnostní aspekty
- Šifrování veškeré komunikace přes Noise protokol (X25519)
- Ed25519 podpisy pro ověření identity (tracker registrace, zprávy)
- Žádný centrální server nedrží zprávy ani metadata
- Decentralizovaná identita: PeerId z veřejného klíče
- Omezení: relay server zná, kdo se připojuje (ale ne obsah zpráv)

= 7. Systémové požadavky a nasazení
- Platforma: UNIX-like systémy (Linux, macOS, BSD)
- Požadavky: Rust toolchain (edition 2024) nebo Nix
- Sestavení: `cargo build` nebo `nix build`
- Spuštění: `nix run` nebo přímé spuštění binárních souborů
- Potřebné komponenty v síti: relay server, HTTP tracker

= 8. Testování
- Popis testovacího prostředí (lokální síť, NAT simulace)
- Scénáře testování:
  - mDNS discovery ve stejné podsíti
  - Komunikace přes relay (různé sítě)
  - Hole-punching (DCUTR) přes NAT
  - Odesílání a příjem zpráv
  - Žádosti o přátelství
  - Setup wizard – registrace jména
- Nalezené problémy a jejich řešení

= 9. Výsledky, diskuse a omezení
- Co bylo úspěšně implementováno
- Aktuální omezení a problémy:
  - Relay circuit listening (nestabilní)
  - Offline doručení zpráv (zatím neimplementováno)
  - Zprávy pouze mezi přáteli (částečně implementováno)
  - Pouze UNIX-like platformy
  - Jeden HTTP tracker (bez redundance)
- Porovnání s cílem práce

= 10. Závěr a budoucí práce
- Shrnutí dosažených výsledků
- Přínos práce
- Budoucí práce:
  - DHT (Kademlia) pro peer discovery bez centrálního trackeru
  - Persistentní offline zprávy přes DHT nebo gossipsub
  - Hlasový chat
  - Skupinové kanály (gossipsub)
  - Konfigurace swarm a síťových parametrů z UI
  - Podpora dalších platforem (Windows)
  - Lepší error handling a graceful shutdown

= Seznam použité literatury (bibliografie)
- libp2p dokumentace a specifikace (QUIC, Noise, Circuit Relay, DCUTR, mDNS)
- RFC dokumenty (RFC 7748 X25519, RFC 6762 mDNS, QUIC RFC)
- Rust dokumentace, Tokio, Ratatui
- Akademické zdroje k P2P sítím a kryptografii

= Přílohy
- A: Ukázky kódu (klíčové části implementace)
  - Konfigurace libp2p Swarm
  - IPC protokol (serializace/deserializace)
  - Databázové schéma (migration-new.sql)
- B: Screenshoty aplikace
  - Setup wizard
  - Hlavní chat rozhraní
  - Friends záložka
- C: Návod k instalaci a spuštění
  - Prerekvizity
  - Kroky sestavení
  - Prvotní konfigurace (setup wizard)
  - Spuštění relay a tracker serverů
- D: Struktura projektu (adresářový strom)
