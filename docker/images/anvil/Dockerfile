FROM ghcr.io/foundry-rs/foundry:latest

RUN apk add --no-cache curl bash

COPY docker/images/anvil/start-anvil.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/start-anvil.sh

EXPOSE 8545

ENTRYPOINT ["/usr/local/bin/start-anvil.sh"]