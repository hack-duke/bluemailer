####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

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


WORKDIR /bluemailer

COPY ./ .

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