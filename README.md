# npipe

Forward named pipe to a TCP address.
```
> npipe --pipe \\.\pipe\foo-pipe --host 127.0.0.1 --port 1234
```

You can filter the accepted address:
```
> npipe --pipe \\.\pipe\foo-pipe --host 0.0.0.0 --port 1234 -f 192.168.1.0/24
```
