CREATE TYPE category_behavior AS ENUM ('fixed', 'variable', 'subscription');
ALTER TABLE category ADD COLUMN behavior category_behavior NULL;
