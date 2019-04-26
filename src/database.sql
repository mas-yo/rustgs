CREATE TABLE `join_room_queue` (
  `user_id` bigint(20) unsigned NOT NULL,
  `room_id` bigint(20) unsigned NOT NULL,
  PRIMARY KEY (`user_id`)
) ENGINE=MEMORY DEFAULT CHARSET=utf8;

CREATE TABLE `rooms` (
  `id` bigint(20) unsigned NOT NULL AUTO_INCREMENT,
  `code` int(11) NOT NULL,
  `player_count` int(11) NOT NULL DEFAULT '0',
  `server_id` int(11) NOT NULL,
  UNIQUE KEY `id` (`id`)
) ENGINE=MEMORY DEFAULT CHARSET=utf8;

CREATE TABLE `servers` (
  `id` bigint(20) unsigned NOT NULL AUTO_INCREMENT,
  `address` varchar(64) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `address` (`address`)
) ENGINE=MEMORY DEFAULT CHARSET=utf8;

CREATE TABLE `users` (
  `id` bigint(20) unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(256) NOT NULL,
  `room_code` int(10) unsigned DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `id` (`id`),
  UNIQUE KEY `name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;
