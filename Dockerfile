####################################################################################################
## Chef
####################################################################################################
FROM rust:latest AS chef
RUN cargo install cargo-chef 

####################################################################################################
## Planner
####################################################################################################
FROM chef AS planner
WORKDIR /bluemailer
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json


####################################################################################################
## Builder
####################################################################################################
FROM chef AS builder
WORKDIR /bluemailer
RUN update-ca-certificates

# Create appuser
# ENV USER=bluemailer
# ENV UID=10001

# RUN adduser \
#     --disabled-password \
#     --gecos "" \
#     --home "/nonexistent" \
#     --shell "/sbin/nologin" \
#     --no-create-home \
#     --uid "${UID}" \
#     "${USER}"



COPY --from=planner /bluemailer/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# We no longer need to use the x86_64-unknown-linux-musl target
RUN cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM gcr.io/distroless/cc-debian12

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /bluemailer

# Copy our build
COPY --from=builder /bluemailer/target/release/bluemailer ./

# Use an unprivileged user.
# USER bluemailer:bluemailer

CMD ["/bluemailer/bluemailer"]