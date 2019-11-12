# Sink /sɪŋk/ 

_Nothing but the kitchen sink_

A new way to syncronize files across systems!

Status: Proof of Concept

## API Design

_typical workflow for a new project_

```console
localhost@user:~$ sink start
daemon started

localhost@user:~$ sink new project # should behave like git init 
cloning project to project, check `~/project/.sink/config.json` for options

localhost@user:~$ cd project
localhost@user:~$ sink open <stream-name> # opens stream to a effective 'branch' of name <stream-name>
syncronize project `~/project` with <stream-name> at <server-location>

localhost@user:~$ vi # do some changes
localhost@user:~$ cat ~/project/.sink/logs/<project-name>.log
path/to/file.txt
+hello, world
-,

localhost@user:~$ sink close <stream-name> # stop consuming resources and watching files
```

# Goals

- Minimal, this project aims to have an extremely minimal api, subcommands * subcommands options will be the formulae for complexity here
- Invisible, We want it to be a fire and forget mentallity, streams will be openable then will simply work.
- Offline, when connections drop the system should calmly wait for connection to be restored and eventually become consistent. Thanks to the nature of online applications it should only matter that visibility is eventually obtained for conflicts.
- Resource concious, users should not know the applications is even running once started
- History, the entire log should be stored and recorded with key events (like a release) being easily identifiable and rollbackable

## Sub Goals

- Fast, the platform should be synced in minutes over days
- Documented errors, errors should provide all the information needed
- Specification over implementation, specifications should be made after PoC phase
- Transparent, everything the application does should be highly visable.
- Secure, currently we will use TLS but security will be more of a consideration going forward.
- Accessible, option of GUI's and other interfaces for users to understand what's going on

