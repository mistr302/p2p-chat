#import "@preview/lilaq:0.5.0" as lq
#import "@preview/codly:1.3.0" as c
// #import "@preview/codly-languages:0.1.1" as cl

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
#set page(paper: "a4", numbering: "1", number-align: right)
#show bibliography: set heading(depth: 1)
#set text(size: 10pt)
#set heading(numbering: "1.")
#show bibliography: set heading(numbering: "1.")
#show heading: set align(center)
#show heading: set block(width: 85%)
#show heading.where(depth: 999): set text(size: 22pt)
#show heading.where(depth: 1): set text(size: 17pt)
#show heading.where(depth: 2): set text(size: 14pt)
#show heading.where(depth: 3): set text(size: 12pt)
#show heading.where(depth: 4): set text(size: 11pt)
#show heading.where(depth: 5): set text(size: 10pt)
// #show raw.where(block: true): set text(size: 9pt)
#show raw: set text(font: "DejaVu Sans Mono")
#show raw.where(block: true): set align(center)
#show table: set align(center)
#show table: set text(size: 9pt, font: "DejaVu Sans Mono")
// #set math.equation(numbering: "1")

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
#outline(title: heading(depth: 1, numbering: none, outlined: false)[Obsah])
#pagebreak()
= Úvod
Chatovací aplikace má fungovat převážně decentralizovaně, což znamená, že každý peer by měl v ideálním případě komunikovat napřímo s tím s kým chce momentálně komunikovat.
== Problém
Aplikace má za cíl řešit problém ochrany soukromí při odesílání zpráv, které by mohly být čteny poskytovateli centralizovaných chatovacích aplikací, a uchovávání metadat, například kdy komunikujete s kým.
== Motivace
== Technologie
Hlavní technologie použité k vytvoření aplikace:
- Rust
- LibP2P
- Tokio (asynchronní runtime)
- Ratatui (uživatelské rozhraní)
- Sqlite (lokální úložiště)
- mDNS (lokální vyhledávání peerů)
- Noise (šifrování komunikace)
- QUIC (hlavní transport protokol)
- KademliaDHT (WAN vyhledávání peerů, data storage)(zatím neimplementováno)

== Klíčové vlastnosti
- Konfigurovatelné TUI s ovládacími prvky podobným jako ve vimu
- Zasílání šifrovaných zpráv napřímo nebo přes DHT(zatím neimplementováno)
- něco

== Postup vývoje
+ Analytická fáze: rešerše podobných existujících platforem a analyzace jejich provedení.
+ Návrhová fáze: vytvoření architektury aplikace, návrhu databázového schématu a uživatelského rozhraní.
+ Implementace: programování aplikace v programovacím jazyce Rust za použití Tokio asynchronous runtime, lokálního databázového uložiště SQLite.
+ Testování: ověřování funkčnosti aplikace.


= Systémové požadavky a omezení

V současné době je aplikace určena pouze pro UNIX-like systémy (Linux, MacOS, BSD).

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
QUIC je nový transportní protokol, který poskytuje vždy šifrované připojení s multiplexováním datových toků postavené na protokolu UDP.@libp2p-quic

QUIC je bezpečný transportní protokol pro všeobecné použití na aplikační vrstvě.

Aplikační protokoly si vyměňují informace přes připojení QUIC prostřednictvím datových toků, které jsou uspořádan sekvence bajtů. 

Připojení QUIC nejsou striktně vázána na jednu síťovou cestu. Migrace připojení používá identifikátory připojení, aby umožnila přenos připojení na novou síťovou cestu.@quicrfc

=== Noise (X25519)
Noise Protocol Framework je široce používaný šifrovací systém, který umožňuje bezpečnou komunikaci kombinováním kryptografických primitiv do vzorů s ověřitelnými bezpečnostními vlastnostmi.@libp2p-noise

Jedná se o rámec pro kryptografické protokoly založený na dohodě o klíči Diffie-Hellman. Noise může popisovat protokoly, které se skládají z jedné zprávy, stejně jako interaktivní protokoly.

Protokol Noise začíná výměnou handshake zpráv mezi dvěma stranami. Během této fáze handshake si strany vymění veřejné klíče DH a provedou sekvenci operací DH, přičemž výsledky DH hashují do sdíleného tajného klíče. Po fázi handshake může každá strana použít tento sdílený klíč k odesílání šifrovaných transportních zpráv.

Rámec Noise podporuje handshake, kde každá strana má dlouhodobý statický pár klíčů a/nebo dočasný pár klíčů.

Všechny zprávy Noise mají délku menší nebo rovno 65535 bajtů.@noiseprotocol

==== X25519
X25519 je funkce eliptické křivky Diffie-Hellman (ECDH), která používá křivku Curve25519. Křivka Curve25519, vyvinutá Danielem J. Bernsteinem v roce 2006, byla navržena tak, aby poskytovala vysokou bezpečnost a výkon a zároveň se vyhýbala běžným úskalím implementace, která se vyskytovala v dřívějších systémech kryptografie eliptických křivek (ECC). X25519, jak je specifikováno v RFC 7748, standardizuje použití Curve25519 pro výměnu klíčů, díky čemuž je široce přijímána v protokolech jako TLS 1.3 a Signal. 

 Ve srovnání s tradičními algoritmy, jako je RSA nebo klasický Diffie-Hellman, nabízí X25519:

 Vyšší bezpečnost podle velikosti klíče – 128bitová bezpečnost s 256bitovými klíči.
 Rychlejší výpočty – zejména na zařízeních s omezenými možnostmi.
 Odolnost proti útokům bočním kanálem – díky jednoduchosti návrhu a implementace.
 Lepší interoperabilita – široká podpora v moderních kryptografických knihovnách.
@x25519
=== Circuit Relay
Circuit relay je transportní protokol, který směruje provoz mezi dvěma peer zařízeními přes třetí stranu „relay“ peer.

V mnoha případech nebudou peer zařízení schopna překonat NAT a/nebo firewall tak, aby byla veřejně přístupná. Nebo nemusí sdílet společné transportní protokoly, které by jim umožňovaly přímou komunikaci.

Aby bylo možné používat architektury peer-to-peer i přes překážky připojení, jako je NAT, definuje libp2p protokol nazvaný p2p-circuit. Pokud peer není schopen naslouchat na veřejné adrese, může se připojit k reléovému peeru, který udrží dlouhodobé připojení otevřené. Ostatní peerové se budou moci připojit přes reléový peer pomocí adresy p2p-circuit, která předá provoz do jeho cíle.

Protokol circuit relay je inspirován TURN, který je součástí sbírky technik NAT traversal Interactive Connectivity Establishment.
@libp2p_circuit_relay
=== Dcutr 
Libp2p DCUtR (Direct Connection Upgrade through Relay) je protokol pro navazování přímých spojení mezi uzly prostřednictvím hole punching, bez signalizačního serveru.
DCUtR zahrnuje synchronizaci a otevírání spojení k předpokládaným externím adresám každého peeru.
@libp2p_dcutr
=== Multicast DNS
mDNS, neboli multicast Domain Name System, je způsob, jakým uzly používají IP multicast k publikování a přijímání DNS záznamů RFC 6762 v rámci lokální sítě.
mDNS se běžně používá v domácích sítích, aby se zařízení jako počítače, tiskárny a chytré televize mohly navzájem objevit a připojit.@libp2p-mdns

Aby mDNS discovery mohl fungovat MUSÍ uzel odesílat své mDNS dotazy z
   portu UDP 5353 a MUSÍ
   naslouchat na odpovědi mDNS odeslané na port UDP 5353 na
   adrese mDNS link-local multicast (224.0.0.251 a/nebo její IPv6
   ekvivalent FF02::FB).@mdnsrfc

= Návrh aplikace

= Implementace
= Vlastnosti a funkce
= Bezpečnostní aspekty
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

#bibliography("ref.bib")
