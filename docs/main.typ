#import "@preview/lilaq:0.5.0" as lq
#import "@preview/codly:1.3.0" as c
// #import "@preview/codly-languages:0.1.1" as cl
#let super-heading(body, size: 18pt) = {
    heading(depth: 1, numbering: none, outlined: false)[#text(size: size)[#body]]
}
#let h1(body, depth: 1, size: 18pt) = {
    heading(depth: 1, numbering: none, outlined: false)[#text(size: size)[#body]]
}
#let h2(body) = {
    h1(body, depth: 2, size: 16pt)
}
#let h3(body) = {
    h1(body, depth: 3, size: 14pt)
}

#set document(
    title: [Komunikační aplikace zaměřující se na soukromí pomocí decentralizace],
    author: "Michal Stránský",
    // description
    keywords: ("p2p", "chat"),
    // date
)
#set text(lang: "cs")

#show: c.codly-init.with()
#c.codly(
    display-name: false,
    breakable: false,
)
#set page(
    paper: "a4",
    numbering: none,
    margin: (x: 2.5cm, y: 3cm),
    // number-align: right,
)
#show bibliography: set heading(depth: 1)
#set text(
    lang: "cs",
    size: 12pt,
    // hyphenate: false,
    costs: (hyphenation: 100%,),
    spacing: 120%,
)
#set heading(numbering: "1.")
#let first-line-indent = false
#set par(
    justify: true,
    leading: 11pt,
    spacing: if first-line-indent { 11pt } else { 20pt },
    first-line-indent: if first-line-indent { (amount: 20pt, all: true) } else { 0pt },
    // first-line-indent: if first-line-indent { 20pt } else { 0pt },
)
// #show heading.where(depth: 1): it => {
//     colbreak(weak: true)
//     it
// }
#show bibliography: set heading(numbering: "1.")
// #show heading: set align(center)
#show heading.where(depth: 1): set text(size: 18pt)
#show heading.where(depth: 2): set text(size: 15pt)
#show heading.where(depth: 3): set text(size: 14pt)
#show heading.where(depth: 4): set text(size: 13pt)
#show heading.where(depth: 5): set text(size: 13pt)
#show heading.where(depth: 1): set block(above: 25pt, below: 20pt)
#show heading.where(depth: 2): set block(above: 25pt, below: 20pt)
#show heading.where(depth: 3): set block(above: 25pt, below: 20pt)
#show heading.where(depth: 4): set block(above: 25pt, below: 20pt)
#show heading.where(depth: 5): set block(above: 25pt, below: 20pt)
// #show raw.where(block: true): set text(size: 9pt)
#show raw: set text(size: 10pt, font: "DejaVu Sans Mono")
#show raw.where(block: true): set align(center)
#show raw.where(block: true): set text(size: 9pt)
#show table: set align(center)
#show table: set text(size: 10pt, font: "DejaVu Sans Mono")

#show math.equation.where(block: false): box
#show cite: box

#show regex("\b[AaIiKkOoSsUuVv] "): it => [#it.text.trim()~] 

#let nobs = sym.space.nobreak

#pagebreak(weak: true)
#pagebreak(weak: true)
#align(center + horizon)[
DELTA – Střední škola informatiky a ekonomie, s.r.o.

Ke Kamenci 151, Pardubice
#image("images/delta-logo.webp", width: 50%, height: 50%, fit: "contain")
#title[
Komunikační aplikace zaměřující se na soukromí pomocí decentralizace
]
]
#align(right)[
Michal Stránský 4.B

Informační technologie (18-20-M/01) 2025/26

pod vedením Ing. Zdeňka Drvoty

Zdokumentováno dne: 2026-01-11
]
#pagebreak()
#h1("Zadání maturitního projektu")

Jméno a příjmení: Michal Stránský  

Pro školní rok: 2025/2026  

Třída: 4. B  

Obor: Informační technologie 18-20-M/01  

Téma práce: Bezpečná P2P komunikační aplikace pro Linux  

Vedoucí práce: Ing. Zdeněk Drvota  

#h2("Cíl práce")

#h3("Vývoj aplikace pro Linux s těmito vlastnostmi:")
- Šifrovaný textový a hlasový chat mezi uživateli
- End-to-end šifrování (E2EE)
- peer-to-peer architektura
- Minimalizace metadat a ochrana soukromí
#h3("Doplňující vzdělávací cíle:")
- Praktické osvojení konceptů kryptografie a síťových technologií

#h2("Očekávané výstupy")

#h3("1. Bezpečnostní vrstva")
- Implementace bezpečného protokolu
- Bezpečná výměna klíčů
- Autentizace uživatelů

#h3("2. Komunikační jádro")
- P2P spojení mezi klienty
- P2P relay server

#h3("3. Uživatelské rozhraní")
- Textové terminálové rozhraní (TUI)
- Základní funkce:
  - Správa kontaktů
  - Šifrovaný textový chat
  - Spuštění hlasového hovoru

#h3("4. Hlasový modul")
- Efektivní komprese audia 

#h2("Funkční požadavky")

#h3("Šifrování")
  - End-to-end šifrování všech dat
  - Ověření identity protistrany

#h3("Síťová komunikace")
  - P2P spojení
  - Odolnost proti změnám síťového prostředí

#h3("Textový chat")
  - Real-time přenos zpráv

#h3("Hlasový chat")
  - Šifrované hovory

#h2("Časový harmonogram")

#h3("Září-Říjen 2025")
- Analýza bezpečnostních protokolů
- Návrh kryptografického modelu
- Studie P2P architektur

#h3("Listopad 2025")
- Implementace šifrovací vrstvy
- Základní síťová komunikace
- Testování kryptografických funkcí

#h3("Prosinec 2025-Leden 2026")
- Vývoj textového rozhraní (TUI)
- Integrace textového chatu
- Uživatelská správa kontaktů

#h3("Únor–Březen 2026")
- Implementace hlasového modulu
- Testování a optimalizace latence
- Penetrační testování

#h3("Duben 2026")
- Komplexní integrační testy
- Dokumentace projektu
- Příprava na obhajobu

#h3("Květen 2026")
- Obhajoba projektu


#h2("Kritéria úspěchu")

1. Veškerá komunikace šifrována end-to-end
2. Funkční P2P spojení
3. Textové rozhraní běžící na Linux


#pagebreak()
#super-heading[Prohlášení]

Prohlašuji, že jsem maturitní projekt vypracoval samostatně, výhradně s použitím uvedené literatury.

#columns(2)[
#align(left)[
V Pardubicích dne 30.3.2026
]
#colbreak()
#align(right)[
$"........................................"$\
Michal Stránský
]
]
#pagebreak()

#outline(title: heading(depth: 1, numbering: none, outlined: false)[Obsah])
#pagebreak()
#heading(numbering: none, outlined: false)[
Poděkování
]
Velmi děkuji panu Ing. Drvotovi za jeho odborné rady, vstřícnost a pomoc, které mi výrazně pomohly při zpracování tohoto projektu.
#pagebreak()
#super-heading[Anotace]
Cílem této práce je vytvořit komunikační aplikaci, která zajišťuje bezpečnost a soukromí při zasílání zpráv mezi uživateli, docílením decentralizace pomocí síťové architektury peer-to-peer(P2P).
V ideálním případě, komunikují uživatelé napřímo mezi sebou, nebo v horším přes relay, kdy relay server slouží jako prostředník a přenáší zprávy mezi uživately, ale nemá přístup k jejich obsahu

#super-heading[Klíčová slova]
sítě, peer-to-peer, p2p, decentralizace, tui, cli, chat, libp2p, rust, async
#super-heading[Abstract]
// TODO: translate the upper annotation
The goal of this project is to create a messaging app that ensures security and privacy when users send messages to one another through decentralization.
Ideally, users communicate directly with one another; otherwise, they communicate via a relay, where the relay server acts as an intermediary and transmits messages between users but cannot read them.
#super-heading[Key words]
networking, peer-to-peer, p2p, decentralization, tui, cli, chat, libp2p, rust, async
#pagebreak()
#set page(numbering: "1")
= Úvod
// TODO: Začít úvodem do peer to peer sítí a jejich architekturou , mozna porovnaní mezi server-client architekturou a moznymi problemy napriklad peer-discovery, hole-punching, navázání spojení,  utoky
Peer-to-peer (P2P) architektura je komputační model, kde jsou si uzly v síti rovni, komunikují a sdílejí informace napřímo mezi sebou. Každý uzel může být klientem i serverem zároveň. Oproti client-server architektuře, kde spoléháme na server, aby přechovával a předal informace a data za nás. Ačkoli je P2P architektura velmi užitečná, přibývají s ní i nějaké překážky, které musíme překonat. Ty jsou například překonání Network Address Translation(NAT) a firewallů, které nám občas znemožní navázat přímé spojení s ostatními uzly. Odpojení uzlů ze sítě, což nám znemožní je kontaktovat a může způsobit ztrátu dat. Bezpečnost a důvěra, některé uzly mohou lhát, vracet špatná koruptovaná data nebo útočit na síť. 
Moje aplikace používá hybridní přístup k P2P architektuře, v síti jsou centrální servery, které přechovávají pouze mapování jmen k identitě uživatele v síti a relay servery které jim poskytují navázat přes ně přímé spojení.
// https://www.pynetlabs.com/peer-to-peer-architecture/
// https://www.geeksforgeeks.org/system-design/peer-to-peer-p2p-architecture/
== Postup vývoje
+ Analytická fáze: rešerše podobných existujících platforem a analýza jejich provedení.
+ Návrhová fáze: vytvoření architektury aplikace, návrhu databázového schématu a uživatelského rozhraní.
+ Implementace: programování aplikace v programovacím jazyce Rust za použití Tokio asynchronous runtime a závislých balíčků, lokálního databázového uložiště SQLite.
+ Testování: ověřování funkčnosti aplikace.

== Aplikace se zkládá z čtyřech hlavních částí:

=== p2pchat-core 
Backendový daemon, který řeší vše ohledně ukládání zpráv, kontaktů, přátel a síťových eventů

=== p2pchat-relay
Relay server, na který se uživatelé připojují pro WAN komunikaci

=== p2pchat-http
HTTP server, přes který si uživatelé registrují uživatelská jména k jejich decentralizované identitě PeerId, pomocí které se dají následně provolat přes relay

=== p2pchat-tui
Uživatelské rozhraní pro komunikaci s p2pchat-core

= Pozadí a analýza existujících řešení

== Problém a motivace
Cílem aplikace je řešit problém ochrany soukromí při odesílání zpráv, které by mohly být čteny poskytovateli centralizovaných chatovacích aplikací, a uchovávání metadat, například kdy komunikujete s kým.

== Stávající chatovací systémy s podobným účelem, ale jiným zpracováním

=== Matrix
- open-source
- každý může hostovat server který se zapojuje do decentralizovaného systému serverů, uživatelé komunikují pomocí těchto serverů //TODO

=== Keet by HolePunch 
- není open-source
- nemá implementaci persistant message storage přes DHT, tudíž přenos zpráv může proběhnout pouze když jsou oba uživatelé přístupní

=== Signal
- open-source
- centralizovaný, šifrovaný end-to-end

= Použité Technologie
== Technologie k samostatnému vytvoření aplikace
=== Rust
Rust je moderní systémový programovací jazyk navržený s ohledem na výkon, spolehlivost a bezpečnost paměti. Původně jej vyvinula společnost Mozilla a jeho cílem je poskytnout vývojářům nízkoúrovňovou kontrolu podobnou jazykům C nebo C++, avšak bez běžných chyb, jako je odkazování na nulový ukazatel. Rust zajišťuje bezpečnost paměti bez použití garbage collectoru díky svému jedinečnému systému vlastnictví a zapůjčování. Klade také důraz na souběžnost, což vývojářům pomáhá psát bezpečné vícevláknové programy bez datových konfliktů.
// TODO sources
=== LibP2P(rust-libp2p)
libp2p je modulární síťová knihovna určená k vytváření peer-to-peer aplikací flexibilním a škálovatelným způsobem. Vznikla v rámci ekosystému Protocol Labs a využívá se v projektech, jako je IPFS. libp2p poskytuje základní stavební kameny pro síťové připojení, včetně transportních protokolů, vyhledávání uzlů, šifrování a multiplexování. Umožňuje vývojářům přizpůsobit způsob, jakým se uzly připojují a komunikují, aniž by byli závislí na centralizovaných serverech.

// TODO sources
=== Tokio (asynchronní runtime)
Tokio je asynchronní runtime pro programovací jazyk Rust, určený k vývoji rychlých a škálovatelných síťových aplikací. Je vyvíjen v rámci projektu Tokio Project a poskytuje nástroje potřebné pro psaní neblokujícího kódu. Tokio obsahuje komponenty, jako je smyčka událostí, plánovač úloh a rozhraní API pro asynchronní vstup a výstup. Umožňuje vývojářům efektivně zpracovávat tisíce souběžných úloh bez blokování vláken.

// TODO sources
=== Ratatui (uživatelské rozhraní)
Ratatui je knihovna pro jazyk Rust určená k vytváření uživatelských rozhraní v terminálu (TUI), která klade důraz na flexibilitu a výkon. Jedná se o komunitní fork projektu tui-rs, na které se i nadále aktivně pracuje a vylepšuje se. Ratatui poskytuje widgety, systémy rozvržení a nástroje pro stylizaci, které umožňují vytvářet interaktivní textová rozhraní v terminálu. Dobře se integruje s asynchronními runtime prostředími, jako je Tokio, a umožňuje tak vytvářet responzivní aplikace. 

// TODO sources
=== Sqlite (tokio-rusqlite)
SQLite je lightweight, samostatný systém pro správu relačních databází, navržený s důrazem na jednoduchost a efektivitu. Vytvořil jej D. Richard Hipp a je široce využíván v embedded systémech a aplikacích. Na rozdíl od tradičních databází nevyžaduje SQLite samostatný server, protože data ukládá přímo do jediného souboru na disku. Podporuje standardní funkce jazyka SQL a zároveň se vyznačuje malými nároky na paměť a vysokou spolehlivostí. 

// TODO sources
=== Nix
Nix je systém pro sestavování a správu balíčků určený k reprodukovatelnému a deklarativnímu sestavování softwaru. Vyvíjí jej nadace NixOS Foundation a tvoří ústřední prvek ekosystému NixOS. Nix využívá čistě funkcionální přístup, v němž jsou výstupy sestavení určovány výhradně vstupy, což zaručuje konzistentní výsledky napříč různými prostředími. Izoluje závislosti, aby se předešlo konfliktům, a umožňuje souběžnou existenci více verzí balíčků. 
// TODO sources
== Protokoly použité k vytvoření aplikace
=== QUIC
// TODO vysvetlit vlastnimy slovy
QUIC je nový transportní protokol, který poskytuje vždy šifrované připojení s multiplexováním datových toků postavené na protokolu UDP.

QUIC je bezpečný transportní protokol pro všeobecné použití na aplikační vrstvě.

Aplikační protokoly si vyměňují informace přes připojení QUIC prostřednictvím datových toků, které jsou uspořádan sekvence bajtů. 

Připojení QUIC nejsou striktně vázána na jednu síťovou cestu. Migrace připojení používá identifikátory připojení, aby umožnila přenos připojení na novou síťovou cestu.
@quicrfc
@libp2p-quic
=== TCP
Transmission Control Protocol
Přenos v segmentech, oproti QUIC(UDP), který je přenášen v datagramech.
Pracuje na transportní vrstvě, je řešen přímo v kernelu.
By default nepodporuje socket multiplexing. // TODO: yamux
TODO
// TODO sources
// TODO
=== HTTP
// TODO sources
TODO
=== Noise
// TODO vysvetlit vlastnimy slovy
Noise Protocol Framework je široce používaný šifrovací systém, který umožňuje bezpečnou komunikaci kombinováním kryptografických primitiv do vzorů s ověřitelnými bezpečnostními vlastnostmi.@libp2p-noise

Jedná se o rámec pro kryptografické protokoly založený na dohodě o klíči Diffie-Hellman. Noise může popisovat protokoly, které se skládají z jedné zprávy, stejně jako interaktivní protokoly.

Protokol Noise začíná výměnou handshake zpráv mezi dvěma stranami. Během této fáze handshake si strany vymění veřejné klíče DH a provedou sekvenci operací DH, přičemž výsledky DH hashují do sdíleného tajného klíče. Po fázi handshake může každá strana použít tento sdílený klíč k odesílání šifrovaných transportních zpráv.

Rámec Noise podporuje handshake, kde každá strana má dlouhodobý statický pár klíčů a/nebo dočasný pár klíčů.

Všechny zprávy Noise mají délku menší nebo rovno 65535 bajtů.@noiseprotocol

==== X25519
// TODO vysvetlit vlastnimy slovy
X25519 je funkce eliptické křivky Diffie-Hellman (ECDH), která používá křivku Curve25519. Křivka Curve25519, vyvinutá Danielem J. Bernsteinem v roce 2006, byla navržena tak, aby poskytovala vysokou bezpečnost a výkon a zároveň se vyhýbala běžným úskalím implementace, která se vyskytovala v dřívějších systémech kryptografie eliptických křivek (ECC). X25519, jak je specifikováno v RFC 7748, standardizuje použití Curve25519 pro výměnu klíčů, díky čemuž je široce přijímána v protokolech jako TLS 1.3 a Signal. 

 Ve srovnání s tradičními algoritmy, jako je RSA nebo klasický Diffie-Hellman, nabízí X25519:

 Vyšší bezpečnost podle velikosti klíče – 128bitová bezpečnost s 256bitovými klíči.
 Rychlejší výpočty – zejména na zařízeních s omezenými možnostmi.
 Odolnost proti útokům bočním kanálem – díky jednoduchosti návrhu a implementace.
 Lepší interoperabilita – široká podpora v moderních kryptografických knihovnách.
@x25519
=== Circuit Relay
// TODO vysvetlit vlastnimy slovy
Circuit relay je transportní protokol, který směruje provoz mezi dvěma peer zařízeními přes třetí stranu „relay“ peer.

V mnoha případech nebudou peer zařízení schopna překonat NAT a/nebo firewall tak, aby byla veřejně přístupná. Nebo nemusí sdílet společné transportní protokoly, které by jim umožňovaly přímou komunikaci.

Aby bylo možné používat architektury peer-to-peer i přes překážky připojení, jako je NAT, definuje libp2p protokol nazvaný p2p-circuit. Pokud peer není schopen naslouchat na veřejné adrese, může se připojit k reléovému peeru, který udrží dlouhodobé připojení otevřené. Ostatní peerové se budou moci připojit přes reléový peer pomocí adresy p2p-circuit, která předá provoz do jeho cíle.

Protokol circuit relay je inspirován TURN, který je součástí sbírky technik NAT traversal Interactive Connectivity Establishment.
@libp2p_circuit_relay
=== Dcutr 
// TODO vysvetlit vlastnimy slovy
Libp2p DCUtR (Direct Connection Upgrade through Relay) je protokol pro navazování přímých spojení mezi uzly prostřednictvím hole punching, bez signalizačního serveru.
DCUtR zahrnuje synchronizaci a otevírání spojení k předpokládaným externím adresám každého peeru.
@libp2p_dcutr
=== Multicast DNS
// TODO vysvetlit vlastnimy slovy
mDNS, neboli multicast Domain Name System, je způsob, jakým uzly používají IP multicast k publikování a přijímání DNS záznamů RFC 6762 v rámci lokální sítě.
mDNS se běžně používá v domácích sítích, aby se zařízení jako počítače, tiskárny a chytré televize mohly navzájem objevit a připojit.@libp2p-mdns

Aby mDNS discovery mohl fungovat MUSÍ uzel odesílat své mDNS dotazy z
   portu UDP 5353 a MUSÍ
   naslouchat na odpovědi mDNS odeslané na port UDP 5353 na
   adrese mDNS link-local multicast (224.0.0.251 a/nebo její IPv6
   ekvivalent FF02::FB).@mdnsrfc

= Návrh aplikace
== Architektura – přehled
- Diagram: TUI ↔ Unix Socket ↔ p2pchat-core ↔ libp2p swarm ↔ peers
- Workspace struktura (Cargo workspace se třemi crates) // TODO přidat relay-server
  - `p2pchat-types` – sdílené typy, pro komunikaci s API
  - `p2pchat-core` – backend daemon
  - `tui` – frontend
- Aplikace je navržena způsobem, aby mohli vývojáři postavit jakékoliv uživatelské rozhraní a napojit ho na API poskytované jádrem mojí aplikace pomocí UNIX socketů.

== IPC komunikace (Unix Socket)
Komunikace mezi jádrem a uživatelským rozhraním probíhá pomocí Unix Socketů.
UI aplikace se připojí na `/tmp/p2p-chat.sock`, kde p2pchat-core poslouchá na navázání komunikace.
Obě strany zapisují eventy prefixované jejich délkou v bajtech.
UI aplikace zapisuje UiClientRequest, který obsahuje req_id a samotný event s payloadem.
Core aplikace zapisuje odpovědi na requesty UiClientResponse s příslušným req_id, který daný event vyvolal.
Zapisuje také eventy, nezávislé na requesty, vyvolané například přijetím zprávy, objevení nového mDNS uzlu, atd.
- Přehled klíčových eventů:
  - SendMessage, LoadChatlogPage
  - SendFriendRequest, AcceptFriendRequest, DenyFriendRequest
  - SearchUsername, LoadFriends, LoadIncomingFriendRequests
  - DiscoverMdnsContact, PeerDisconnected, RelayServerConnection

== Databázové schéma
- Tabulky a vztahy:
  - `contacts` – PeerId, vazba na names a channel
  - `names` – jméno s TTL (24 h), podporuje jména potvrzená centrálním serverem, nebo předané uživatelem
  - `channels` – privátní kanály mezi uživateli 1:1
  - `messages` – UUID, obsah, channel_id, created_at
  - `friends` – potvrzená přátelství
  - `pending_friend_requests` – čekající žádosti (incoming / outgoing)
#image("images/DB_schema.jpg", fit: "contain")

== Identita a kryptografie
- Ed25519 klíčový pár jako identita uživatele 
- PeerId odvozeno z veřejného klíče pomocí multihashe base58 encoded
- Podpisování zpráv pro ověření autenticity (`signable.rs`)
- HTTP tracker: registrace jména s podepsaným požadavkem

== Konfigurace a datové uložiště
Veškerá konfigurace programu se ukládá v souboru `$XDG_CONFIG_HOME/p2pchat/settings` nebo `$HOME/.config/p2pchat/settings` ve formátu JSON. Momentálně musí povinně obsahovat platné hodnoty username a keypair
Databázový soubor SQLite se nachází v souboru `$XDG_DATA_HOME/p2pchat/db`.
Při prvním spuštění, tedy v případě, že neexistuje konfigurační soubor, spustíme binární soubor tui/setup kde si vygenerujeme ed25519 keypair registrujeme jméno na trackeru


== Klíčové vlastnosti
- Konfigurovatelné TUI s ovládacími prvky podobným jako ve vimu
- // TODO další věci

= Implementace
== p2pchat-core (backend daemon)
- Vstupní bod `main.rs`: Unix socket server, dispatcher eventů
- CLI argumenty: `-r` relay adresa, `-t` tracker adresa
=== Síťová vrstva (`network.rs`)
- Konfigurace libp2p Swarm (behavior stack)
- EventLoop: zpracování swarm eventů a příkazů z TUI
- Client: async API pro posílání příkazů do EventLoop
- Buffering požadavků při nedostupnosti peera

=== Protokoly
- `/direct-message/1.0.0` – Request-Response protokol pro zprávy
  - `DirectMessageRequest` (obsah zprávy)
  - `MessageResponse` (Ack / NotFriends)
- `/friends/1.0.0` – protokol pro správu přátelství a name resolving
  - `FriendRequest::AddFriend` – žádost o přátelství
  - `FriendRequest::AcceptFriend { decision }` – přijetí/odmítnutí
  - Přenos jména peera při mDNS discovery
=== Databázová vrstva (`db/`)
- CRUD operace pro contacts, messages, friends
- Stránkování historie zpráv

== tui/ (terminálové UI)
- Dva binární cíle: `tui` (hlavní UI) a `setup` (wizard)

=== Setup wizard (`setup.rs`)
Toto uživatelské rozhraní slouží k prvotnímu nastavení aplikace a ke změně uživatelského jména na HTTP trackeru.
Při prvním spuštění se vygeneruje ed25519 KeyPair identita uživatele.
Uživatel zadává jeho požadované uživatelské jméno do bufferu, který automaticky po 1 vteřině nečinnosti odesílá na HTTP tracker API, GET request, aby zjistil, zda je uživatelské jméno dostupné a propisuje stav do rozhraní.
Uživatel následně může dostupné uživatelské jméno zaregistrovat stlačením enter.
Po nastavení všech požadovaných možností, uživatel nastavení uloží, čímž ukončí setup proces.

=== Hlavní UI (`main.rs`, `ui.rs`)
Uživatelské rozhraní samotné aplikace se skládá z levého sidebaru, obsahové oblasti a dolního status baru.
Rozhraní obsahuje dvě záložky, jednu na přímou komunikaci v kanálech s uživateli a druhou na správu přátel.
Sidebar obsahuje TabSwitch, který nám říká na jaké záložce se nacházíme a seznam všech kanálů.
Obsahová oblast zobrazuje rozhraní na komunikaci (chatlog, chat input) nebo správu stavu přátelství uživatelů a vyhledávání nových uživatelů, dle vybrané záložky.
Status bar zobrazuje naše jméno a aktuální stav připojení k relay serveru.

== HTTP Tracker klient (`tracker.rs`)
Tento klient řeší zasílání registračních POST requestů na tracker server s ed25519 podpisem.
Vyhledání jména přes GET request na http tracker a kontrolu dostupností jmen při registraci.

// == Běh programu
// Program se nastaví dle konfiguračního souboru
// Poslouchá na UNIX_SOCKET /tmp/p2p-chat.sock
// Po navázání spojení, čte z UNIX_SOCKET UiClientRequesty, dle kterých zasílá dotazy jiným uživatelům, sqlite dotazy, http-tracker, ..
// Odpovědi na tyto eventy zárověň zapisuje na stejný socket, jako enum WriteEvent.
// Zapisuje také eventy, nezávislé na UiClientRequest, např. mDNS discover, IncomingMessage, DcutrConnection(hole-punch success) ..
// Také může nastat stav, kdy se nepodaří programu nastartovat, např. když se nepodaří načíst nastavení programu,
// v tomto příkladě zapíše WriteEvent::CriticalError a proces se ukončí.

== Provolání(Dial) vzdáleného uživatele
- Pošleme dotaz na překlad jména na HTTP-tracker, který nám vrátí PeerId
- Pošleme dotaz na CircuitRelay zda je tento uživatel připojený
- Pokud ano pokusíme se navázat spojení
- Pokud se vše povede, máme spojení přes Relay
- Následně se pokusíme o přímě spojení pomocí Dcutr

= Bezpečnostní aspekty
Veškerá síťová komunikace v rámcí aplikace je šífrována pomocí Noise protokolu.
Identita každého uživatele je vedena decentralizovaně pomocí PeerId, která je vygenerována multihash a base58 algoritmem se vstupem veřejného klíče uživatele.
Ověření identity odesílatele zpráv je implementováno pomocí Ed25519 podpisů např. v registraci nového uživatelského jména na HTTP trackeru.
Žádný centrální server nedrží zprávy ani metadata. 
HTTP server jako takový je pouze HTTP a samostatně nepodporuje šifrování komunikace, šifrování HTTP komunikace přes TLS se řeší na úrovni hostingu nejčasteji použitím reverse-proxy služby jako Nginx, Caddy, atd.
HTTP klient v p2pchat-core je schopný HTTPS šifrované komunikace. 
- Omezení: relay server zná, kdo se připojuje (ale ne obsah zpráv)

= Systémové požadavky a omezení
V současné době je aplikace určena pouze pro UNIX-like systémy (Linux, MacOS, BSD).
Požadavky pro úspěšnou kompilaci programu: Rust toolchain (edice 2024) s nainstalovanými závislostmi definovanými v souboru flake.nix nebo Nix package manager
== Omezení
Ke komunikaci s uživateli, kteří nejsou dostupní na lokální síti, je třeba mít relay a http tracker, dostupný z veřejného internetu, na který se všichni uživatelé budou připojovat. 
// = Návrh aplikace
// = Implementace
// = Vlastnosti a funkce
// - Každý uživatel si nezávisle vede vlastní tabulky přátel, přijatých a odeslaných zpráv

= Výsledky, diskuse a omezení
Výsledkem projektu je aplikace s funkčním TUI prostředím, která umožňuje jednoduché spojení na LAN prostředí pomocí mDNS a zároveň vyhledávání vzdálených uživatelů a následné komunikování přes Relay nebo napřímo pomocí hole-punchingu.
== Možné budoucí rozšíření projektu
Současná aplikace funguje ve spolehlivě, avšak má několik nedostatků, které je třeba vyřešit. V současné verzi, není možné poslat zprávu, bez toho aby oba dva uživatelé byly dostupní. Tento jev, by byl možný vyřešit například implementací DHT(Distrubuted Hash Table) nebo gossipsub mezi společnými přáteli, kteří by si navzájem drželi nedoručené zprávy.
V momentálním stavu aplikace neřeší a nepočítá s žádnými útoky, například náš HTTP tracker, může být obětí útoku Sybil, kde by útočník mohl zabrat velké množství jmen, jen tím že bude měnit svoji identitu// TODO
- Hlasový chat
- Ukládání zpráv pro peery, kteří se dlouho nepřipojí k DHT (Nebo použít gossipsub kde si přátelé předávají zprávy offline přátelům)
- Systém pro zpracování jmen peerů (odvození hash pro DHT Node ID?) nebo pomocí trackerů
- atd.

= Závěr
TODO
#pagebreak()
#set page(numbering: none)
// = Závěr a budoucí práce
// == Budoucí práce
// - hlasový chat
// - konfigurace swarmu a sítových eventů
// - 

#bibliography("ref.bib")
