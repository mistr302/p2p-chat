# Project Overview

This repository contains an asynchronous Rust chatting application focused on decentralisation
The application consists of the application frontend in TUI. Most of the network is handled using libp2p.
The TUI side should post and pull events from UNIX socket setup in /p2pchat-core/src/main.rs and handle them accordingly

## Setup

Install dependencies:
nix develop

Run the p2p-chat core:
cargo r 2>trace.log
Read the log:
cat trace.log

Run the p2p-chat tui:
cargo r


## Coding Guidelines

- Use Rust for all files
- Follow best practices

## Project Structure

/src
 /tui & tui.rs => Contains TUI Types, Event handling and the UI itself
 main.rs => the entry point of the TUI application
 /network => handles incoming, outgoing traffic for individual protocols
 network.rs => Contains swarm configuration, network events, Client and Eventloop implementation and network event handling
 settings.rs => Contains logic for loading and saving settings and creating save files
 /db => Contains db config, migration and migration script

