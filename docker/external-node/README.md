### To perform the following operations, go to the root directory of the project.

```shell
mkdir cargo
cp -r ~/.cargo/bin cargo
mkdir cargo/registry
cp -r ~/.cargo/registry/index cargo/registry/
cp -r ~/.cargo/registry/cache cargo/registry/
mkdir cargo/git/
cp -r ~/.cargo/git/db cargo/git/
```

### Execute in the root directory of the project.（Root Directory: $MICRO_HOME (Replace with your own translation)）

```shell
docker build -t ext-node -f $MICRO_HOME/docker/external-node/Dockerfile-2 .
```

### Replace <image> with your own image name

### 1.Modify your private key in user.dev.

user.dev

### 2.Initializing Execution

```shell
docker run -v $MICRO_HOME/docker/external-node/user.env:/etc/env/user.env ext-node database setup
```

### 3.Run the program

```shell
docker run -v $MICRO_HOME/docker/external-node/user.env:/etc/env/user.env -p 3060:3060 -p 3062:3062 ext-node
```

### Note: The copy project added in Dockerfile-2 file needs to be checked if it is allowed in the .dockerignore file.
