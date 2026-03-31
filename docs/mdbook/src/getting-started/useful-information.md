# Useful infos

After installation, you can edit the configuration file and start the service. If you run through the
installer over the shell script then all should be set and ready to go.

If you need to edit the config file it is under `/etc/observer/observer.toml`.

### IMPORTANT

If you need to reconfigure or update observer you can do this via the provided
shellcript on our repo or webseite.

For starting the service or other information
(this is all for systemd you can search the correspoding commands
for you system or if you have an own config it may be different):

```sh
# get the status of observer
systemctl status observer

# restart
sytemctl restart observer

# see and follow the logs
journalctl -u observer -f

# show the last 20 log lines
journalctl -u observer -n 20
```
