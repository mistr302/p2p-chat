# Project Overview

This repository contains an asynchronous Rust chatting application focused on decentralisation
The application consists of the application frontend in TUI. Most of the network is handled using libp2p.

## Setup

Install dependencies:
nix-shell

Run the application:
cargo r 2>trace.log

Read the log:
cat trace.log

## Coding Guidelines

- Use Rust for all files
- Follow best practices

## Project Structure

/src
 /tui & tui.rs => Contains TUI Types, Event handling and the UI itself
 main.rs => the entry point of the application
 /network => handles incoming, outgoing traffic for individual protocols
 network.rs => Contains swarm configuration, network events, Client and Eventloop implementation and network event handling
 settings.rs => Contains logic for loading and saving settings and creating save files
 /db => Contains db config, migration and migration script

