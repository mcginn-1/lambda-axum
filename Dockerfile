FROM public.ecr.aws/lambda/provided:al2 as builder

# Install compiler and Rust
RUN yum install -y gcc gcc-c++ openssl-devel
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a new empty shell project
WORKDIR /usr/src/app
COPY . .

# Build the release
RUN rustup target add x86_64-unknown-linux-musl && \
    cargo build --release --target x86_64-unknown-linux-musl

# Copy the built executable to the Lambda base image
FROM public.ecr.aws/lambda/provided:al2
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/lambda-hello-world /var/runtime/bootstrap
CMD [ "bootstrap" ]
