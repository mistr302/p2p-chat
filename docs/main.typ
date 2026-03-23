#import "@preview/lilaq:0.5.0" as lq
#import "@preview/codly:1.3.0" as c
// #import "@preview/codly-languages:0.1.1" as cl
#let super-heading(body, size: 18pt) = {
    heading(depth: 1, numbering: none, outlined: false)[#text(size: size)[#body]]
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
= Úvod
Cílem této práce je vytvořit komunikační aplikaci, která zajišťuje bezpečnost a soukromí při zasílání zpráv mezi uživateli, pomocí decentralizace.
Aplikace se zkládá z čtyřech hlavních částí:
- p2pchat-core = // TODO
- p2pchat-relay = // TODO
- p2pchat-http = // TODO
- p2pchat-tui = // TODO
V ideálním případě, komunikují uživatelé napřímo mezi sebou, nebo v horším přes RelayedConnection, kdy relay server slouží jako prostředník a přenáší zprávy mezi uživately
== Problém
Aplikace má za cíl řešit problém ochrany soukromí při odesílání zpráv, které by mohly být čteny poskytovateli centralizovaných chatovacích aplikací, a uchovávání metadat, například kdy komunikujete s kým.
== Motivace
== Technologie
Hlavní technologie použité k vytvoření aplikace:
=== Rust
Rust je moderní systémový programovací jazyk navržený s ohledem na výkon, spolehlivost a bezpečnost paměti. Původně jej vyvinula společnost Mozilla a jeho cílem je poskytnout vývojářům nízkoúrovňovou kontrolu podobnou jazykům C nebo C++, avšak bez běžných chyb, jako je odkazování na nulový ukazatel. Rust zajišťuje bezpečnost paměti bez použití garbage collectoru díky svému jedinečnému systému vlastnictví a zapůjčování. Klade také důraz na souběžnost, což vývojářům pomáhá psát bezpečné vícevláknové programy bez datových konfliktů. Celkově si Rust klade za cíl kombinovat rychlost, bezpečnost a produktivitu vývojářů při vytváření všeho od operačních systémů po webové aplikace.
=== LibP2P(rust-libp2p)
libp2p je modulární síťová knihovna určená k vytváření peer-to-peer aplikací flexibilním a škálovatelným způsobem. Vznikla v rámci ekosystému Protocol Labs a využívá se v projektech, jako je IPFS. libp2p poskytuje základní stavební kameny pro síťové připojení, včetně transportních protokolů, vyhledávání uzlů, šifrování a multiplexování. Umožňuje vývojářům přizpůsobit způsob, jakým se uzly připojují a komunikují, aniž by byli závislí na centralizovaných serverech. Celkově si libp2p klade za cíl umožnit decentralizované, odolné a interoperabilní síťové systémy.
=== Tokio (asynchronní runtime)
Tokio je asynchronní runtime pro programovací jazyk Rust, určený k vývoji rychlých a škálovatelných síťových aplikací. Je vyvíjen v rámci projektu Tokio Project a poskytuje nástroje potřebné pro psaní neblokujícího kódu. Tokio obsahuje komponenty, jako je smyčka událostí, plánovač úloh a rozhraní API pro asynchronní vstup a výstup. Umožňuje vývojářům efektivně zpracovávat tisíce souběžných úloh bez blokování vláken. Celkově si Tokio klade za cíl usnadnit vývoj vysoce výkonných souběžných systémů v jazyce Rust.
=== Ratatui (uživatelské rozhraní)
Ratatui je knihovna pro jazyk Rust určená k vytváření bohatých uživatelských rozhraní v terminálu (TUI), která klade důraz na flexibilitu a výkon. Jedná se o komunitní odnož projektu tui-rs, na které se i nadále aktivně pracuje a vylepšuje se. Ratatui poskytuje widgety, systémy rozvržení a nástroje pro stylizaci, které umožňují vytvářet interaktivní textová rozhraní v terminálu. Dobře se integruje s asynchronními runtime prostředími, jako je Tokio, a umožňuje tak vytvářet responzivní aplikace. Celkově si Ratatui klade za cíl usnadnit navrhování moderních, uživatelsky přívětivých terminálových aplikací v Rustu.
=== Sqlite
SQLite je lightweight, samostatný systém pro správu relačních databází, navržený s důrazem na jednoduchost a efektivitu. Vytvořil jej D. Richard Hipp a je široce využíván v embedded systémech a aplikacích. Na rozdíl od tradičních databází nevyžaduje SQLite samostatný server, protože data ukládá přímo do jediného souboru na disku. Podporuje standardní funkce jazyka SQL a zároveň se vyznačuje malými nároky na paměť a vysokou spolehlivostí. Celkově si SQLite klade za cíl poskytnout snadno použitelné, přenositelné a bezkonfigurační databázové řešení.
// Lightweight serverless databázové uložiště.
// Funguje jako normální relační databáze.
// Ukládá se jako soubor nebo může být pouze in-memory
// === Noise (šifrování komunikace)
// === QUIC (hlavní transport protokol)
// === Http tracker server
=== Nix
Nix je systém pro sestavování a správu balíčků určený k reprodukovatelnému a deklarativnímu sestavování softwaru. Vyvíjí jej nadace NixOS Foundation a tvoří ústřední prvek ekosystému NixOS. Nix využívá čistě funkcionální přístup, v němž jsou výstupy sestavení určovány výhradně vstupy, což zaručuje konzistentní výsledky napříč různými prostředími. Izoluje závislosti, aby se předešlo konfliktům, a umožňuje souběžnou existenci více verzí balíčků. Celkově si Nix klade za cíl poskytovat spolehlivá, opakovatelná a snadno sdíletelná vývojová a nasazovací prostředí.

== Klíčové vlastnosti
- Konfigurovatelné TUI s ovládacími prvky podobným jako ve vimu
- Zasílání šifrovaných zpráv napřímo nebo přes DHT(zatím neimplementováno)
- něco

== Postup vývoje
+ Analytická fáze: rešerše podobných existujících platforem a analyzace jejich provedení.
+ Návrhová fáze: vytvoření architektury aplikace, návrhu databázového schématu a uživatelského rozhraní.
+ Implementace: programování aplikace v programovacím jazyce Rust za použití Tokio asynchronous runtime a závislých balíčků, lokálního databázového uložiště SQLite.
+ Testování: ověřování funkčnosti aplikace.

== Běh programu
Program se nastaví dle konfiguračního souboru
Poslouchá na UNIX_SOCKET /tmp/p2p-chat.sock
Po navázání spojení, čte z UNIX_SOCKET UiClientEventy, dle kterých provolává uživatele, sqlite dotazy, http-tracker, ..
Odpovědi na tyto eventy zárověň zapisuje na stejný socket, jako enum WriteEvent.
Zapisuje také eventy, nezávislé na UiClientEvent, např. mDNS discover, IncomingMessage, DcutrConnection(hole-punch success) ..
Také může nastat stav, kdy se nepodaří programu nastartovat, např. když se nepodaří načíst nastavení programu,
v tomto příkladě zapíše WriteEvent::CriticalError a proces se ukončí.

=== Provolání(Dial) vzdáleného uživatele
- Pošleme dotaz na překlad jména na HTTP-tracker, který nám vrátí PeerId
- Pošleme dotaz na CircuitRelay zda je tento uživatel připojený
- Pokud ano pokusíme se navázat spojení
- Pokud se vše povede, máme spojení přes Relay
- V ideálním případě se nám podaří Dcutr, a máme přímé spojení, tudíž už nemusíme komunikovat přes Relay k tomuto uživateli

= Systémové požadavky a omezení
V současné době je aplikace určena pouze pro UNIX-like systémy (Linux, MacOS, BSD).
Jde zkompilovat pomocí "cargo build", ale musíte mít nainstalované všechny závislosti definované v flake.nix
Nebo jde vybuildit pomocí nix build systému pomocí příkazu: "nix build" nebo "nix run" pro spuštění 

= Pozadí
== Stávající chatovací systémy s podobným účelem, ale jiným zpracováním
=== Matrix
- open-source
- každý může hostovat server který se zapojuje do decentralizovaného systému serverů, uživatelé komunikují pomocí těchto serverů //TODO
=== Keet by HolePunch 
- není open-source
- nemá implementaci persistant message storage přes DHT, tudíž přenos zpráv může proběhnout pouze když jsou oba uživatelé přístupní
== Protokoly
=== QUIC
// TODO vysvetlit vlastnimy slovy
QUIC je nový transportní protokol, který poskytuje vždy šifrované připojení s multiplexováním datových toků postavené na protokolu UDP.@libp2p-quic

QUIC je bezpečný transportní protokol pro všeobecné použití na aplikační vrstvě.

Aplikační protokoly si vyměňují informace přes připojení QUIC prostřednictvím datových toků, které jsou uspořádan sekvence bajtů. 

Připojení QUIC nejsou striktně vázána na jednu síťovou cestu. Migrace připojení používá identifikátory připojení, aby umožnila přenos připojení na novou síťovou cestu.@quicrfc
=== TCP
Transmission Control Protocol
Přenos v segmentech, oproti QUIC(UDP), který je přenášen v datagramech.
Pracuje na transportní vrstvě, je řešen přímo v kernelu.
By default nepodporuje socket multiplexing. // TODO: yamux
// TODO
=== HTTPS
==== TLS 1.3
=== Noise (X25519)
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
Aplikace je navržena způsobem, aby mohli vývojáři postavit jakékoliv uživatelské rozhraní a napojit ho na API poskytované jádrem mojí aplikace pomocí UNIX socketů.
= Implementace
= Vlastnosti a funkce
- Každý uživatel si nezávisle vede vlastní tabulky přátel, přijatých a odeslaných zpráv

= Bezpečnostní aspekty
- Veškerá komunikace přes mDNS, Circuit Relay a přímá hole-punched komunikace je šifrovaná pomocí Noise protokolu
= Výsledky, diskuse a omezení
Stávající problémy, které je třeba vyřešit:
- Ukládání zpráv pro peer, kteří se dlouho nepřipojí k DHT (Nebo použít gossipsub kde si přátelé předávají zprávy offline přátelům)
- Systém pro zpracování jmen peerů (odvození hash pro DHT Node ID?) nebo pomocí trackerů
- atd.
= Závěr a budoucí práce
== Budoucí práce
- hlasový chat
- konfigurace swarmu a sítových eventů
- 

#pagebreak()
#bibliography("ref.bib")
