ALTER TABLE alerts
    ADD CONSTRAINT alerts_assigned_membership_fk
        FOREIGN KEY (operator_id, assigned_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id)
        NOT VALID;

ALTER TABLE alerts
    VALIDATE CONSTRAINT alerts_assigned_membership_fk;

ALTER TABLE alerts
    DROP CONSTRAINT alerts_assigned_identity_fk;

ALTER TABLE alert_actions
    ADD CONSTRAINT alert_actions_assigned_membership_fk
        FOREIGN KEY (operator_id, assigned_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id)
        NOT VALID;

ALTER TABLE alert_actions
    VALIDATE CONSTRAINT alert_actions_assigned_membership_fk;

ALTER TABLE alert_actions
    DROP CONSTRAINT alert_actions_assigned_identity_id_fkey;
