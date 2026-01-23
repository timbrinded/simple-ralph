# `simple ralph`

![simple-ralph](simple-ralph.png)

## Overview

This is a CLI application for working on ralph loops.

## Features

## Requirements

- Working claude code
- A PRD JSON file

### PRD Specification

As recommended by the excellent article from Anthropic [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents), the PRD is a long standard JSON file with tasks reminscent of a KanBan board, if you are familiar with that flavour of Agile development.

```json
{
    "category": "functional",
    "description": "New chat button creates a fresh conversation",
    "steps": [
      "Navigate to main interface",
      "Click the 'New Chat' button",
      "Verify a new conversation is created",
      "Check that chat area shows welcome state",
      "Verify conversation appears in sidebar"
    ],
    "passes": false
  }

```

## Installation

## Usage
