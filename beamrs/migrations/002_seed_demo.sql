INSERT INTO frequencies (name)
VALUES ('general'), ('support'), ('announcements')
ON CONFLICT (name) DO NOTHING;

INSERT INTO users (username)
VALUES ('Anon42'), ('ui-team'), ('ops')
ON CONFLICT (username) DO NOTHING;

INSERT INTO rays (frequency_id, user_id, text)
SELECT f.id, u.id, 'The beam is strong today.'
FROM frequencies f
JOIN users u ON u.username = 'Anon42'
WHERE f.name = 'general'
ON CONFLICT DO NOTHING;

INSERT INTO rays (frequency_id, user_id, text)
SELECT f.id, u.id, 'Shipping the new release notes.'
FROM frequencies f
JOIN users u ON u.username = 'ui-team'
WHERE f.name = 'announcements'
ON CONFLICT DO NOTHING;

INSERT INTO prisms (user_id, ray_id)
SELECT prism_user.id, r.id
FROM users prism_user
JOIN users author ON author.username = 'Anon42'
JOIN rays r ON r.user_id = author.id
WHERE prism_user.username = 'ops'
ON CONFLICT (user_id, ray_id) DO NOTHING;
