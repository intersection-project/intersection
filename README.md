# The Intersection Project

TODO: Signal handlers so the docker runner doesn't need `--init`

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
