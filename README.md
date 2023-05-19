<div align="center">

# The Intersection Project - Discord mentions, supercharged.

</div>

Intersection is a Discord bot and software suite that empowers Discord's @-mentions with advanced
concepts with their roots in set theory, such as taking the **intersection** of two roles.

TODO: Signal handlers so the docker runner doesn't need `--init`

**BIG TODO:** WHEN WE GO OPEN SOURCE, add the repo url to Cargo.toml and update /version to link to the current git ref using build_info::PKG_REPOSITORY

## Setup/Deploy for Docker LogN-side

using our fancy local repo

**locally, run:**

```bash
docker build -t intersection .
docker tag intersection 10.0.0.2:5000/intersection
docker push 10.0.0.2:5000/intersection
```

**on the target machine (ct), run:** (needed file 'env' in cwd)

```bash
docker pull 10.0.1.114:443/intersection
docker stop intersection
docker rm intersection
docker run \
    -d \
    --restart unless-stopped \
    --init \
    --env-file=env \
    --name intersection \
    10.0.1.114:443/intersection
```
