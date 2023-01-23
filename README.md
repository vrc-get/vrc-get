OSS VPM
====

Opensource reimplementation of vpm command.

## Goals

- Provide OSS version of vpm command especially around package manager
- Provide more functionality than official vpm around package manager

## Progress

Ports

- [ ] `vpm new`
- [x] `vpm add`
  - [x] `vpm add repo`
  - [x] `vpm add package`
- [ ] `vpm install`
  - [ ] `vpm install hub` (Not to be implemented early)
  - [ ] `vpm install unity` (Not to be implemented early)
  - [ ] `vpm install vcc` (Not to be implemented early)
  - [ ] `vpm install templates` (I don't know how can I implement this)
- [ ] `vpm list`
  - [ ] `vpm list unity` (Not to be implemented early)
  - [x] `vpm list repos`
  - [x] `vpm list repo`
  - [ ] `vpm list templates` (Not to be implemented early)
- [ ] `vpm check` (Not to be implemented early)
  - [ ] `vpm check package`
  - [ ] `vpm check unity`
  - [ ] `vpm check hub`
  - [ ] `vpm check project`
  - [ ] `vpm check template`
  - [ ] `vpm check vcc`
- [ ] `vpm remove`
  - [ ] `vpm remove repos`
- [ ] `vpm migrate`
  - [ ] `vpm migrate project`

Features

- [x] `vpm add repo <pkg> <version>`
- [x] `vpm resolve` An command to resolve packages

